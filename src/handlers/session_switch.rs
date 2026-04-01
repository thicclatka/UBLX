//! In-process switch to another indexed root (same terminal session; no `exec`).

use std::path::PathBuf;
use std::sync::mpsc;

use crate::app::{RunUblxParams, RunUblxStartupFlow, load_snapshot_for_tui};
use crate::config::{
    UblxOpts, UblxOptsForDirExtras, UblxPaths, ensure_global_config_file_with_defaults,
    record_ublx_session_open,
};
use crate::engine::{db_ops, orchestrator};
use crate::handlers::{core, snapshot};
use crate::layout::setup;
use crate::themes;
use crate::utils::BumperBuffer;

/// Replace the running session’s indexed root: new DB, new opts, new background snapshot channel, fresh UI state.
///
/// Caller should clear/redraw the terminal after success if desired.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on error.
pub fn perform_session_switch<'a>(
    new_dir: PathBuf,
    params: &mut RunUblxParams<'a>,
    ublx_opts: &mut UblxOpts,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    state: &mut setup::UblxState,
    bumper: Option<&'a BumperBuffer>,
) -> Result<(), String> {
    let dir = new_dir;
    let db_path = db_ops::ensure_ublx_and_db(&dir).map_err(|e| e.to_string())?;

    let paths = UblxPaths::new(&dir);
    let cached_settings = db_ops::load_settings_from_db(&db_path).ok().flatten();
    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let for_dir_config = UblxOptsForDirExtras {
        valid_theme_names: &valid_themes,
        bumper,
    };
    *ublx_opts = UblxOpts::for_dir(
        &dir,
        &paths,
        None,
        None,
        None,
        cached_settings.as_ref(),
        &for_dir_config,
    );

    let (tx, rx) = mpsc::channel::<(usize, usize, usize)>();
    let tx_for_tui = tx.clone();
    let dir_clone = dir.clone();
    let opts_clone = ublx_opts.clone();
    let prior_clone = db_ops::load_nefax_from_db(&dir_clone, &db_path)
        .ok()
        .flatten();
    let bumper_for_thread = bumper.cloned();
    std::thread::spawn(move || {
        snapshot::run_snapshot_pipeline(
            &dir_clone,
            &opts_clone,
            prior_clone.as_ref(),
            Some(tx),
            bumper_for_thread.as_ref(),
        );
    });

    let config_reload_rx = Some(core::spawn_config_watcher(&dir));
    let lens_names = db_ops::load_lens_names(&db_path).unwrap_or_default();
    let pending_force_full_enhance_toast = orchestrator::should_force_full_zahir(ublx_opts);
    let _ = record_ublx_session_open(&dir);

    let (right_pane_tx, right_pane_rx) =
        tokio::sync::mpsc::unbounded_channel::<setup::RightPaneAsyncReady>();

    params.db_path = db_path;
    params.dir_to_ublx = dir;
    params.snapshot_done_rx = Some(rx);
    params.snapshot_done_tx = Some(tx_for_tui);
    params.bumper = bumper;
    params.theme.clone_from(&ublx_opts.theme);
    params.layout = ublx_opts.layout.clone();
    params.bg_opacity = ublx_opts.bg_opacity.unwrap_or(1.0);
    params.opacity_format = ublx_opts.opacity_format;
    params.duplicate_groups.clear();
    params.duplicate_mode = db_ops::DuplicateGroupingMode::NameSize;
    params.duplicate_groups_rx = None;
    params.lens_names = lens_names;
    params.config_reload_rx = config_reload_rx;
    params.startup = RunUblxStartupFlow {
        defer_first_snapshot: false,
        pending_force_full_enhance_toast,
    };
    params.right_pane_async_tx = Some(right_pane_tx);

    let (c, r) = load_snapshot_for_tui(
        &params.db_path,
        db_ops::SnapshotReaderPreference::PreferUblx,
    );
    *categories = c;
    *all_rows = r;

    *state = setup::UblxState::new();
    state.right_pane_async.rx = Some(right_pane_rx);
    {
        let paths = UblxPaths::new(params.dir_to_ublx.as_path());
        if let Some(g) = paths.global_config() {
            ensure_global_config_file_with_defaults(
                &g,
                themes::default_theme_for_new_config_file(),
            );
        }
    }
    state.snapshot_bg.done_received = !categories.is_empty() || !all_rows.is_empty();

    Ok(())
}
