use log::{debug, error};
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crate::config::{OPERATION_NAME, RunMode, UblxOpts, UblxPaths};
use crate::engine::{db_ops, orchestrator};
use crate::handlers::nefax_ops;
use crate::utils::notifications::BumperBuffer;

/// Run snapshot pipeline in test mode (no TUI).
/// Returns `Err` on failure.
pub fn run_test_mode(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
    start_time: Option<Instant>,
) -> Result<(), anyhow::Error> {
    let mode = RunMode::from_opts(ublx_opts);
    match mode {
        RunMode::Sequential => {
            if let Err(e) = orchestrator::run_sequential(dir_to_ublx, ublx_opts, prior_nefax) {
                error!("sequential mode failed: {}", e);
                std::process::exit(1);
            }
        }
        RunMode::Stream => {
            if let Err(e) = orchestrator::run_stream(dir_to_ublx, ublx_opts, prior_nefax) {
                error!("stream mode failed: {}", e);
                std::process::exit(1);
            }
        }
    }
    if let Err(e) = db_ops::UblxCleanup::new(dir_to_ublx).post_run_cleanup() {
        error!("failed to cleanup: {}", e);
        std::process::exit(1);
    }
    let duration = start_time.unwrap().elapsed();
    debug!(
        "UBLX test completed in {:.4?} seconds",
        duration.as_secs_f64()
    );
    Ok(())
}

/// Run snapshot pipeline (orchestrator + cleanup), signal on `done_tx` when finished.
/// On pipeline error, logs and optionally pushes to `bumper`; still signals done.
pub fn run_snapshot_pipeline(
    dir: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
    done_tx: Option<mpsc::Sender<()>>,
    bumper: Option<BumperBuffer>,
) {
    let mode = RunMode::from_opts(ublx_opts);
    let run_result = match mode {
        RunMode::Sequential => orchestrator::run_sequential(dir, ublx_opts, prior_nefax),
        RunMode::Stream => orchestrator::run_stream(dir, ublx_opts, prior_nefax),
    };
    if let Err(e) = run_result {
        error!("take snapshot failed: {}", e);
        if let Some(b) = bumper.as_ref() {
            b.push_with_operation(
                log::Level::Error,
                format!("Snapshot failed: {}", e),
                Some(OPERATION_NAME.snapshot()),
            );
        }
    } else if let Err(e) = db_ops::UblxCleanup::new(dir).post_run_cleanup() {
        error!("post-run cleanup failed: {}", e);
    }
    if let Some(tx) = done_tx {
        let _ = tx.send(());
    }
}

/// Load prior_nefax and ublx_opts from `dir` and `db_path`, then run [run_snapshot_pipeline].
/// Use from the TUI when running a snapshot on demand (e.g. Shift+S).
pub fn run_snapshot_pipeline_from_dir_db(
    dir: &Path,
    db_path: &Path,
    done_tx: Option<mpsc::Sender<()>>,
    bumper: Option<BumperBuffer>,
) {
    let prior_nefax = db_ops::load_nefax_from_db(dir, db_path).ok().flatten();
    let cached = db_ops::load_settings_from_db(db_path).ok().flatten();
    let paths = UblxPaths::new(dir);
    let ublx_opts = UblxOpts::for_dir(dir, &paths, None, None, None, cached.as_ref());
    run_snapshot_pipeline(dir, &ublx_opts, &prior_nefax, done_tx, bumper);
}

/// Push "Snapshot finished" and, if the latest snapshot has changes, a summary line to the bumper.
pub fn push_snapshot_done_to_bumper(bumper: &BumperBuffer, db_path: &Path) {
    bumper.push_with_operation(
        log::Level::Info,
        "Snapshot finished".into(),
        Some(OPERATION_NAME.snapshot()),
    );
    let (added, mod_count, removed) = count_snapshot_changes(db_path);
    if added + mod_count + removed > 0 {
        let summary = format!(
            "{} added, {} modified, {} removed",
            added, mod_count, removed
        );
        bumper.push_with_operation(log::Level::Info, summary, Some(OPERATION_NAME.snapshot()));
    }
}

/// Count added, modified, and removed changes in the latest snapshot.
/// Returns (added, modified, removed) counts.
fn count_snapshot_changes(db_path: &Path) -> (usize, usize, usize) {
    if let Ok(Some((added, mod_count, removed))) = db_ops::load_delta_log_latest_counts(db_path) {
        (added, mod_count, removed)
    } else {
        (0, 0, 0)
    }
}
