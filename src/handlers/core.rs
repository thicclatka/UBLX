//! Top-level run dispatch: test mode (no TUI) or TUI with background snapshot pipeline.
//! TUI setup/teardown (terminal, raw mode) lives here; the main loop lives in [crate::layout::event_loop::main_app_loop].

use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crossterm::cursor::Show as ShowCursor;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::config::UblxOpts;
use crate::engine::db_ops::SnapshotReaderPreference;
use crate::handlers::{nefax_ops::NefaxResult, snapshot};
use crate::layout::{event_loop, setup};
use crate::utils::notifications;

/// Parameters for [run_app]. Build after DB and opts are ready.
pub struct RunAppParams<'a> {
    pub test_mode: bool,
    pub dir_to_ublx: &'a Path,
    pub db_path: &'a Path,
    pub ublx_opts: &'a UblxOpts,
    pub prior_nefax: &'a Option<NefaxResult>,
    pub bumper: Option<&'a notifications::BumperBuffer>,
    pub dev: bool,
    pub start_time: Option<Instant>,
}

/// Run the app in the selected mode: test (snapshot only, exit) or TUI with background pipeline.
/// Returns `Err` on test failure or TUI error.
pub fn run_app(params: RunAppParams<'_>) -> std::io::Result<()> {
    match params.test_mode {
        true => run_test_mode(
            params.dir_to_ublx,
            params.ublx_opts,
            params.prior_nefax,
            params.start_time,
        ),
        false => run_tui_mode(
            params.dir_to_ublx,
            params.db_path,
            params.ublx_opts,
            params.prior_nefax,
            params.bumper,
            params.dev,
        ),
    }
}

fn run_test_mode(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<NefaxResult>,
    start_time: Option<Instant>,
) -> std::io::Result<()> {
    snapshot::run_test_mode(dir_to_ublx, ublx_opts, prior_nefax, start_time)
        .map_err(|e| std::io::Error::other(e.to_string()))
}

fn run_tui_mode(
    dir_to_ublx: &Path,
    db_path: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<NefaxResult>,
    bumper: Option<&notifications::BumperBuffer>,
    dev: bool,
) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel::<(usize, usize, usize)>();
    let tx_for_tui = tx.clone();
    let dir_clone = dir_to_ublx.to_path_buf();
    let opts_clone = ublx_opts.clone();
    let prior_clone = prior_nefax.clone();
    std::thread::spawn(move || {
        snapshot::run_snapshot_pipeline(&dir_clone, &opts_clone, &prior_clone, Some(tx), None);
    });

    let mut params = event_loop::RunUblxParams {
        db_path,
        dir_to_ublx,
        snapshot_done_rx: Some(rx),
        snapshot_done_tx: Some(tx_for_tui),
        bumper,
        dev,
        theme: ublx_opts.theme.clone(),
        transparent: ublx_opts.transparent,
        duplicate_groups: Vec::new(),
        duplicate_groups_rx: None,
    };
    run_ublx(&mut params)?;
    if let Some(b) = bumper
        && dev
    {
        notifications::flush_bumper_to_stderr(b);
    }
    Ok(())
}

/// Restore terminal to cooked mode, leave alternate screen, show cursor. Used on normal exit and from panic hook.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let mut out = io::stdout();
    let _ = crossterm::execute!(out, LeaveAlternateScreen, ShowCursor);
}

/// Setup terminal, run [crate::layout::event_loop::main_app_loop], then teardown. Called by [run_tui_mode].
/// A panic hook restores the terminal on panic so the shell stays usable.
pub fn run_ublx(params: &mut event_loop::RunUblxParams<'_>) -> io::Result<()> {
    let (mut categories, mut all_rows) =
        event_loop::load_snapshot_for_tui(params.db_path, SnapshotReaderPreference::PreferUblx);
    let mut state = setup::UblxState::new();
    // Already-done dir: we have data, skip polling to avoid redundant first-tick load (stutter).
    if !categories.is_empty() || !all_rows.is_empty() {
        state.snapshot_done_received = true;
    }

    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    let result = event_loop::main_app_loop(
        &mut terminal,
        &mut state,
        &mut categories,
        &mut all_rows,
        params,
    );

    restore_terminal();
    terminal.show_cursor()?;
    result
}
