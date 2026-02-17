//! Top-level run dispatch: test mode (no TUI) or TUI with background snapshot pipeline.

use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crate::handlers::nefax_ops::NefaxResult;
use crate::handlers::snapshot;
use crate::layout::event_loop::run_ublx;
use crate::utils::notifications;

use crate::config::UblxOpts;

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
    let (tx, rx) = mpsc::channel();
    let tx_for_tui = tx.clone();
    let dir_clone = dir_to_ublx.to_path_buf();
    let opts_clone = ublx_opts.clone();
    let prior_clone = prior_nefax.clone();
    std::thread::spawn(move || {
        snapshot::run_snapshot_pipeline(&dir_clone, &opts_clone, &prior_clone, Some(tx), None);
    });

    run_ublx(
        db_path,
        dir_to_ublx,
        Some(rx),
        Some(tx_for_tui),
        bumper,
        dev,
    )?;
    if let Some(b) = bumper
        && dev
    {
        notifications::flush_bumper_to_stderr(b);
    }
    Ok(())
}
