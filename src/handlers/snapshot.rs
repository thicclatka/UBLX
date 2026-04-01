use log::{debug, error};
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crate::config::{OPERATION_NAME, UblxOpts, UblxOptsForDirExtras, UblxPaths};
use crate::engine::{db_ops, orchestrator};
use crate::fatal;
use crate::integrations::NefaxResult;
use crate::themes;
use crate::utils::BumperBuffer;

/// Run snapshot pipeline headlessly (no TUI), e.g. `--snapshot-only`.
/// Returns `Err` on failure.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when the orchestrator or follow-up steps fail.
pub fn run_snapshot_only(
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
            "UBLX snapshot-only completed in {:.4?} seconds",
            t.elapsed().as_secs_f64()
        );
    } else {
        debug!("UBLX snapshot-only completed");
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
                let op = OPERATION_NAME.op("snapshot");
                b.remove_messages_for_operation(&op);
                let err_msg = format!("Snapshot failed: {e}");
                b.push_with_operation(log::Level::Error, err_msg.as_str(), Some(op));
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
/// Use from the TUI when running a snapshot on demand (e.g. Command Mode: Ctrl+A, then s).
///
/// `preserve_*_cache_before_apply`: when `Some`, replaces the corresponding field on the opts returned from
/// [`UblxOpts::for_dir`]. Pass the in-memory opts from the running TUI so a second `for_dir` in the snapshot thread
/// still sees a false→true flip after [`UblxOpts::reload_hot_config`] (or startup `for_dir`) already wrote the new
/// values to the on-disk cache — otherwise the snapshot would look like "no flip" (e.g. skip full Zahir, or lose hash
/// backfill intent).
pub fn run_snapshot_pipeline_from_dir_db(
    dir: &Path,
    db_path: &Path,
    done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    bumper: Option<&BumperBuffer>,
    preserve_enhance_all_cache_before_apply: Option<Option<bool>>,
    preserve_with_hash_cache_before_apply: Option<Option<bool>>,
) {
    let prior_nefax = db_ops::load_nefax_from_db(dir, db_path).ok().flatten();
    let cached = db_ops::load_settings_from_db(db_path).ok().flatten();
    let paths = UblxPaths::new(dir);
    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let for_dir_config = UblxOptsForDirExtras {
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
    if let Some(v) = preserve_with_hash_cache_before_apply {
        ublx_opts.with_hash_cache_before_apply = v;
    }
    run_snapshot_pipeline(dir, &ublx_opts, prior_nefax.as_ref(), done_tx, bumper);
}

/// Spawn a thread that runs [`run_snapshot_pipeline_from_dir_db`]. Use from the TUI when the user triggers a snapshot (e.g. Command Mode: Ctrl+A, then s).
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
        let preserve_with_hash_cache_before_apply =
            preserve_enhance_cache_from.map(|o| o.with_hash_cache_before_apply);
        std::thread::spawn(move || {
            run_snapshot_pipeline_from_dir_db(
                &dir,
                &db,
                Some(tx_clone),
                bumper_clone.as_ref(),
                preserve_enhance_all_cache_before_apply,
                preserve_with_hash_cache_before_apply,
            );
        });
    }
}

/// Push "Snapshot finished" and a second line: counts when there are changes, "No changes" at Info when there are not.
/// Replaces any prior snapshot-tagged lines in the bumper so only this completion is visible.
pub fn push_snapshot_done_to_bumper(
    bumper: &BumperBuffer,
    added: usize,
    mod_count: usize,
    removed: usize,
) {
    let op = OPERATION_NAME.op("snapshot");
    bumper.remove_messages_for_operation(&op);
    bumper.push_with_operation(log::Level::Info, "Snapshot finished", Some(op.clone()));
    if added + mod_count + removed > 0 {
        let summary = format!("{added} added, {mod_count} modified, {removed} removed");
        bumper.push_with_operation(log::Level::Info, summary.as_str(), Some(op));
    } else {
        bumper.push_with_operation(log::Level::Info, "No changes", Some(op));
    }
}
