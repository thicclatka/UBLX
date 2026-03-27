//! Top-level run dispatch: test mode (no TUI) or TUI with background snapshot pipeline.
//! TUI setup/teardown (terminal, raw mode) lives here; the main loop lives in [`crate::app::main_loop`].

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crossterm::{
    cursor::Show as ShowCursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use log::debug;
use notify::{RecursiveMode, Watcher};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::app;
use crate::config::{
    UblxOpts, UblxPaths, ensure_global_config_file_with_defaults, record_ublx_session_open,
};
use crate::engine::{
    db_ops::{SnapshotReaderPreference, load_lens_names},
    orchestrator,
};
use crate::handlers::{applets::first_run, snapshot};
use crate::integrations::NefaxResult;
use crate::layout::setup;
use crate::themes::default_theme_for_new_config_file;
use crate::utils::{BumperBuffer, flush_bumper_to_stderr};

/// Parameters for [`run_app`]. Build after DB and opts are ready.
pub struct RunAppParams<'a> {
    pub test_mode: bool,
    pub dir_to_ublx: &'a Path,
    pub db_path: &'a Path,
    pub ublx_opts: &'a mut UblxOpts,
    pub prior_nefax: Option<&'a NefaxResult>,
    pub bumper: Option<&'a BumperBuffer>,
    pub dev: bool,
    pub start_time: Option<Instant>,
    /// Show first-run welcome when [`crate::config::paths::should_show_initial_prompt`] is true (see its doc).
    pub initial_prompt: bool,
}

/// Run the app in the selected mode: test (snapshot only, exit) or TUI with background pipeline.
///
/// # Errors
///
/// Returns [`io::Error`] from test mode, TUI setup, or the main run loop (terminal I/O).
pub fn run_app(params: &mut RunAppParams<'_>) -> std::io::Result<()> {
    if params.test_mode {
        run_test_mode(
            params.dir_to_ublx,
            params.ublx_opts,
            params.prior_nefax,
            params.start_time,
        )
    } else {
        run_tui_mode(
            params.dir_to_ublx,
            params.db_path,
            params.ublx_opts,
            params.prior_nefax,
            params.bumper,
            params.dev,
            params.initial_prompt,
        )
    }
}

fn run_test_mode(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: Option<&NefaxResult>,
    start_time: Option<Instant>,
) -> std::io::Result<()> {
    snapshot::run_test_mode(dir_to_ublx, ublx_opts, prior_nefax, start_time)
        .map_err(|e| std::io::Error::other(e.to_string()))
}

fn run_tui_mode(
    dir_to_ublx: &Path,
    db_path: &Path,
    ublx_opts: &mut UblxOpts,
    prior_nefax: Option<&NefaxResult>,
    bumper: Option<&BumperBuffer>,
    dev: bool,
    initial_prompt: bool,
) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel::<(usize, usize, usize)>();
    let tx_for_tui = tx.clone();
    if !initial_prompt {
        let dir_clone = dir_to_ublx.to_path_buf();
        let opts_clone = ublx_opts.clone();
        let prior_clone = prior_nefax.cloned();
        std::thread::spawn(move || {
            snapshot::run_snapshot_pipeline(
                &dir_clone,
                &opts_clone,
                prior_clone.as_ref(),
                Some(tx),
                None,
            );
        });
    }

    let config_reload_rx = Some(spawn_config_watcher(dir_to_ublx));

    let lens_names = load_lens_names(db_path).unwrap_or_default();
    let pending_force_full_enhance_toast =
        !initial_prompt && orchestrator::should_force_full_zahir(ublx_opts);
    if !initial_prompt {
        let _ = record_ublx_session_open(dir_to_ublx);
    }
    let mut params = app::RunUblxParams {
        db_path,
        dir_to_ublx,
        snapshot_done_rx: Some(rx),
        snapshot_done_tx: Some(tx_for_tui),
        bumper,
        display: app::RunUblxDisplayOpts { dev },
        theme: ublx_opts.theme.clone(),
        layout: ublx_opts.layout.clone(),
        duplicate_groups: Vec::new(),
        duplicate_groups_rx: None,
        lens_names,
        config_reload_rx,
        startup: app::RunUblxStartupFlow {
            defer_first_snapshot: initial_prompt,
            pending_force_full_enhance_toast,
        },
    };
    run_ublx(&mut params, ublx_opts)?;
    if let Some(b) = bumper
        && dev
    {
        flush_bumper_to_stderr(b);
    }
    Ok(())
}

/// Restore terminal to cooked mode, leave alternate screen, show cursor. Used on normal exit and from panic hook.
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    let mut out = io::stdout();
    let _ = crossterm::execute!(out, DisableMouseCapture, LeaveAlternateScreen, ShowCursor);
}

/// Leave alternate screen and raw mode so an external editor runs on the main screen; call before spawning the editor.
///
/// # Errors
///
/// Returns [`io::Error`] from crossterm when disabling raw mode or manipulating the terminal.
pub fn leave_terminal_for_editor() -> io::Result<()> {
    disable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, DisableMouseCapture, LeaveAlternateScreen, ShowCursor)?;
    Ok(())
}

/// Re-enter alternate screen and raw mode after the editor exits, so the TUI can redraw.
///
/// # Errors
///
/// Returns [`io::Error`] from crossterm when enabling raw mode or manipulating the terminal.
pub fn reapply_terminal_after_editor() -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

/// Setup terminal, run [`crate::app::main_loop`], then teardown. Called by [`run_tui_mode`].
/// A panic hook restores the terminal on panic so the shell stays usable.
///
/// # Errors
///
/// Returns [`io::Error`] from terminal setup, the main loop, or teardown (raw mode, alternate screen, draw).
pub fn run_ublx(params: &mut app::RunUblxParams<'_>, ublx_opts: &mut UblxOpts) -> io::Result<()> {
    let (mut categories, mut all_rows) =
        app::load_snapshot_for_tui(params.db_path, SnapshotReaderPreference::PreferUblx);
    let mut state = setup::UblxState::new();
    {
        let paths = UblxPaths::new(params.dir_to_ublx);
        if let Some(g) = paths.global_config() {
            ensure_global_config_file_with_defaults(&g, default_theme_for_new_config_file());
        }
    }
    if params.startup.defer_first_snapshot {
        first_run::init_prompt_state(&mut state, params.dir_to_ublx);
    }
    debug!(
        "clipboard copy command: {}",
        state
            .clipboard_copy
            .as_ref()
            .map_or_else(|| "(none)".to_owned(), |c| c.argv.join(" "))
    );
    // Already-done dir: we have data, skip polling to avoid redundant first-tick load (stutter).
    if !categories.is_empty() || !all_rows.is_empty() {
        state.snapshot_bg.done_received = true;
    }
    // First-run prompt defers the background snapshot; treat as idle for scheduling (not "snapshot in flight").
    if params.startup.defer_first_snapshot {
        state.snapshot_bg.done_received = true;
    }

    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    let result = app::main_loop(
        &mut terminal,
        &mut state,
        &mut categories,
        &mut all_rows,
        params,
        ublx_opts,
    );

    restore_terminal();
    terminal.show_cursor()?;
    result
}

/// Debounce window: only send at most one reload signal per this many ms (avoids triplicate from multiple notify events per save).
const CONFIG_WATCH_DEBOUNCE_MS: u64 = 600;
/// Watcher thread parks for this many seconds when idle (long sleep so the thread stays alive without busy-looping).
const CONFIG_WATCHER_PARK_SECS: u64 = 86400;

/// Spawns a thread that watches global and local config paths; sends `()` when a config file changes so the main loop can trigger hot reload. Debounced so one save yields one signal. If the watcher cannot be created, the thread exits silently (no reload signals).
fn spawn_config_watcher(dir_to_ublx: &Path) -> mpsc::Receiver<()> {
    let paths = UblxPaths::new(dir_to_ublx);
    let global = paths.global_config();
    let dir = dir_to_ublx.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let last_send_ms = Arc::new(AtomicU64::new(0));

    std::thread::spawn(move || {
        let paths = UblxPaths::new(&dir);
        let global_clone = global.clone();
        let last_send = Arc::clone(&last_send_ms);
        let Ok(mut watcher) = notify::recommended_watcher(move |res: Result<notify::Event, _>| {
            if let Ok(e) = res {
                for p in &e.paths {
                    let is_config =
                        paths.is_config_file(p) || global_clone.as_ref().is_some_and(|g| g == p);
                    if is_config {
                        let now_ms = u64::try_from(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis(),
                        )
                        .unwrap_or(u64::MAX);
                        if now_ms.saturating_sub(last_send.load(Ordering::Relaxed))
                            > CONFIG_WATCH_DEBOUNCE_MS
                        {
                            last_send.store(now_ms, Ordering::Relaxed);
                            let _ = tx.send(());
                        }
                        break;
                    }
                }
            }
        }) else {
            return;
        };
        if watcher.watch(&dir, RecursiveMode::NonRecursive).is_err() {
            return;
        }
        if let Some(ref g) = global {
            let _ = watcher.watch(g, RecursiveMode::NonRecursive);
        }
        loop {
            std::thread::sleep(std::time::Duration::from_secs(CONFIG_WATCHER_PARK_SECS));
        }
    });

    rx
}
