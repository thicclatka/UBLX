use log::{debug, error};
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths};
use crate::engine::{db_ops, orchestrator};
use crate::fatal;
use crate::integrations::NefaxResult;
use crate::themes;
use crate::utils::notifications::BumperBuffer;

/// Run snapshot pipeline in test mode (no TUI).
/// Returns `Err` on failure.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when the orchestrator or follow-up steps fail.
pub fn run_test_mode(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: Option<&NefaxResult>,
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
    if let Some(t) = start_time {
        debug!(
            "UBLX test completed in {:.4?} seconds",
            t.elapsed().as_secs_f64()
        );
    } else {
        debug!("UBLX test completed");
    }
    Ok(())
}

/// Run snapshot pipeline (orchestrator + cleanup), send (added, modified, removed) on `done_tx` when finished.
/// On pipeline error, logs and optionally pushes to `bumper`; still signals done with (0, 0, 0).
pub fn run_snapshot_pipeline(
    dir: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: Option<&NefaxResult>,
    done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    bumper: Option<&BumperBuffer>,
) {
    let counts = match orchestrator::run(dir, ublx_opts, prior_nefax) {
        Ok(c) => c,
        Err(e) => {
            error!("take snapshot failed: {e}");
            if let Some(b) = bumper.as_ref() {
                let err_msg = format!("Snapshot failed: {e}");
                b.push_with_operation(
                    log::Level::Error,
                    err_msg.as_str(),
                    Some(OPERATION_NAME.op("snapshot")),
                );
            }
            (0, 0, 0)
        }
    };

    if let Err(e) = db_ops::UblxCleanup::new(dir).post_run_cleanup() {
        error!("post-run cleanup failed: {e}");
    }
    if let Some(tx) = done_tx {
        let _ = tx.send(counts);
    }
}

/// Load `prior_nefax` and `ublx_opts` from `dir` and `db_path`, then run [`run_snapshot_pipeline`].
/// Use from the TUI when running a snapshot on demand (e.g. Ctrl+S).
///
/// `preserve_enhance_all_cache_before_apply`: when `Some`, replaces [`UblxOpts::enable_enhance_all_cache_before_apply`]
/// on the opts returned from [`UblxOpts::for_dir`]. Pass the in-memory opts from the running TUI (e.g. main's startup
/// `for_dir`) so index-time Zahir still treats a false→true flip as needing a full run: a second `for_dir` in this
/// process re-reads the on-disk cache after the first `for_dir` already wrote `enable_enhance_all = true`, which
/// would otherwise look like "no flip" and skip full Zahir.
pub fn run_snapshot_pipeline_from_dir_db(
    dir: &Path,
    db_path: &Path,
    done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    bumper: Option<&BumperBuffer>,
    preserve_enhance_all_cache_before_apply: Option<Option<bool>>,
) {
    let prior_nefax = db_ops::load_nefax_from_db(dir, db_path).ok().flatten();
    let cached = db_ops::load_settings_from_db(db_path).ok().flatten();
    let paths = UblxPaths::new(dir);
    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let for_dir_config = crate::config::ForDirConfig {
        valid_theme_names: &valid_themes,
        bumper,
    };
    let mut ublx_opts = UblxOpts::for_dir(
        dir,
        &paths,
        None,
        None,
        None,
        cached.as_ref(),
        &for_dir_config,
    );
    if let Some(v) = preserve_enhance_all_cache_before_apply {
        ublx_opts.enable_enhance_all_cache_before_apply = v;
    }
    run_snapshot_pipeline(dir, &ublx_opts, prior_nefax.as_ref(), done_tx, bumper);
}

/// Spawn a thread that runs [`run_snapshot_pipeline_from_dir_db`]. Use from the TUI when the user triggers a snapshot (e.g. Ctrl+S).
pub fn spawn_snapshot_from_dir_db(
    dir: &Path,
    db_path: &Path,
    done_tx: Option<&mpsc::Sender<(usize, usize, usize)>>,
    bumper: Option<&BumperBuffer>,
    preserve_enhance_cache_from: Option<&UblxOpts>,
) {
    if let Some(tx) = done_tx {
        let dir = dir.to_path_buf();
        let db = db_path.to_path_buf();
        let tx_clone = tx.clone();
        let bumper_clone = bumper.cloned();
        let preserve_enhance_all_cache_before_apply =
            preserve_enhance_cache_from.map(|o| o.enable_enhance_all_cache_before_apply);
        std::thread::spawn(move || {
            run_snapshot_pipeline_from_dir_db(
                &dir,
                &db,
                Some(tx_clone),
                bumper_clone.as_ref(),
                preserve_enhance_all_cache_before_apply,
            );
        });
    }
}

/// Push "Snapshot finished" and a second line: counts when there are changes, "No changes" at Info when there are not.
pub fn push_snapshot_done_to_bumper(
    bumper: &BumperBuffer,
    added: usize,
    mod_count: usize,
    removed: usize,
) {
    bumper.push_with_operation(
        log::Level::Info,
        "Snapshot finished",
        Some(OPERATION_NAME.op("snapshot")),
    );
    if added + mod_count + removed > 0 {
        let summary = format!("{added} added, {mod_count} modified, {removed} removed");
        bumper.push_with_operation(
            log::Level::Info,
            summary.as_str(),
            Some(OPERATION_NAME.op("snapshot")),
        );
    } else {
        bumper.push_with_operation(
            log::Level::Info,
            "No changes",
            Some(OPERATION_NAME.op("snapshot")),
        );
    }
}
