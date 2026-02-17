use log::{debug, error};
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths};
use crate::engine::{db_ops, orchestrator};
use crate::fatal;
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
    fatal!(
        orchestrator::run(dir_to_ublx, ublx_opts, prior_nefax),
        "pipeline failed: {}"
    );
    fatal!(
        db_ops::UblxCleanup::new(dir_to_ublx).post_run_cleanup(),
        "failed to cleanup: {}"
    );
    let duration = start_time.unwrap().elapsed();
    debug!(
        "UBLX test completed in {:.4?} seconds",
        duration.as_secs_f64()
    );
    Ok(())
}

/// Run snapshot pipeline (orchestrator + cleanup), send (added, modified, removed) on `done_tx` when finished.
/// On pipeline error, logs and optionally pushes to `bumper`; still signals done with (0, 0, 0).
pub fn run_snapshot_pipeline(
    dir: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
    done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    bumper: Option<BumperBuffer>,
) {
    let counts = match orchestrator::run(dir, ublx_opts, prior_nefax) {
        Ok(c) => c,
        Err(e) => {
            error!("take snapshot failed: {}", e);
            if let Some(b) = bumper.as_ref() {
                b.push_with_operation(
                    log::Level::Error,
                    format!("Snapshot failed: {}", e),
                    Some(OPERATION_NAME.snapshot()),
                );
            }
            (0, 0, 0)
        }
    };
    if let Err(e) = db_ops::UblxCleanup::new(dir).post_run_cleanup() {
        error!("post-run cleanup failed: {}", e);
    }
    if let Some(tx) = done_tx {
        let _ = tx.send(counts);
    }
}

/// Load prior_nefax and ublx_opts from `dir` and `db_path`, then run [run_snapshot_pipeline].
/// Use from the TUI when running a snapshot on demand (e.g. Shift+S).
pub fn run_snapshot_pipeline_from_dir_db(
    dir: &Path,
    db_path: &Path,
    done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    bumper: Option<BumperBuffer>,
) {
    let prior_nefax = db_ops::load_nefax_from_db(dir, db_path).ok().flatten();
    let cached = db_ops::load_settings_from_db(db_path).ok().flatten();
    let paths = UblxPaths::new(dir);
    let ublx_opts = UblxOpts::for_dir(dir, &paths, None, None, None, cached.as_ref());
    run_snapshot_pipeline(dir, &ublx_opts, &prior_nefax, done_tx, bumper);
}

/// Push "Snapshot finished" and, if this run had changes, a summary line to the bumper.
pub fn push_snapshot_done_to_bumper(
    bumper: &BumperBuffer,
    added: usize,
    mod_count: usize,
    removed: usize,
) {
    bumper.push_with_operation(
        log::Level::Info,
        "Snapshot finished".into(),
        Some(OPERATION_NAME.snapshot()),
    );
    if added + mod_count + removed > 0 {
        let summary = format!(
            "{} added, {} modified, {} removed",
            added, mod_count, removed
        );
        bumper.push_with_operation(log::Level::Info, summary, Some(OPERATION_NAME.snapshot()));
    }
}
