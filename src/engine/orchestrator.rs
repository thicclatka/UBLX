use log::{debug, error};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::config::{UblxOpts, UblxPaths};
use crate::engine::db_ops;
use crate::handlers::nefax_ops;
use crate::handlers::zahir_ops;
use crate::utils::{canonicalize_dir_to_ublx, error_writer};

type PreRunSetup = (PathBuf, std::collections::HashMap<String, String>);

fn pre_run_setup(dir_to_ublx: &Path) -> PreRunSetup {
    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let db_path = UblxPaths::new(dir_to_ublx).db();
    let prior_zahir_json = db_ops::load_snapshot_zahir_json_map(&db_path).unwrap_or_default();
    (dir_to_ublx_abs, prior_zahir_json)
}

/// Log nefax error and exit. Use in both sequential and stream error paths.
fn on_nefax_error(dir_to_ublx: &Path, e: &impl std::fmt::Display) -> ! {
    let _ = error_writer::write_nefax_error_to_log(dir_to_ublx, e);
    error!("nefax failed: {}", e);
    std::process::exit(1);
}

/// Run nefaxer; on success return `(nefax, diff)`, on error log and exit. Use when no cleanup is needed (e.g. sequential).
fn run_nefax_exiting<F>(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: Option<&nefax_ops::NefaxResult>,
    entry_callback: Option<F>,
) -> (nefax_ops::NefaxResult, nefax_ops::NefaxDiff)
where
    F: FnMut(&nefax_ops::NefaxEntry),
{
    match nefax_ops::run_nefaxer(dir_to_ublx, ublx_opts, prior_nefax, entry_callback) {
        Ok(result) => result,
        Err(e) => on_nefax_error(dir_to_ublx, &e),
    }
}

/// Paths that need zahir this run: non-empty files whose mtime changed or are new.
fn paths_needing_zahir(
    nefax: &nefax_ops::NefaxResult,
    prior_nefax: Option<&nefax_ops::NefaxResult>,
    dir_to_ublx_abs: &Path,
) -> Vec<PathBuf> {
    nefax
        .iter()
        .filter(|(path, meta)| {
            meta.size > 0 && zahir_ops::needs_zahir(prior_nefax, path, meta.mtime_ns)
        })
        .map(|(p, _)| dir_to_ublx_abs.join(p))
        .collect()
}

pub fn run_sequential(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
) -> io::Result<()> {
    let (dir_to_ublx_abs, prior_zahir_json) = pre_run_setup(dir_to_ublx);
    let (nefax, diff) = run_nefax_exiting::<fn(&nefax_ops::NefaxEntry)>(
        dir_to_ublx,
        ublx_opts,
        prior_nefax.as_ref(),
        None,
    );

    debug!(
        "indexed {} paths (added: {}, removed: {}, modified: {})",
        nefax.len(),
        diff.added.len(),
        diff.removed.len(),
        diff.modified.len()
    );
    let path_list = paths_needing_zahir(&nefax, prior_nefax.as_ref(), &dir_to_ublx_abs);

    debug!(
        "zahir running on {} paths (mtime changed or new)",
        path_list.len()
    );
    let zahir_result = match zahir_ops::run_zahir_batch(&path_list, ublx_opts) {
        Ok(r) => r,
        Err(e) => {
            error!("zahir (sequential) failed: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = error_writer::write_zahir_failures_to_log(dir_to_ublx, &zahir_result) {
        error!("failed to write zahir failures to ublx.log: {}", e);
    }
    if let Err(e) = db_ops::write_snapshot_to_db(
        dir_to_ublx,
        &nefax,
        &zahir_result,
        &diff,
        &ublx_opts.to_ublx_settings(),
        &prior_zahir_json,
    ) {
        error!("failed to write snapshot: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

pub fn run_stream(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
) -> io::Result<()> {
    let (dir_to_ublx_abs, prior_zahir_json) = pre_run_setup(dir_to_ublx);

    let ublx_opts_for_zahir = ublx_opts.clone();
    let (path_tx, path_rx) = mpsc::channel();
    let (output_tx, output_rx) = mpsc::channel();
    let zahir_handle = std::thread::spawn(move || {
        zahir_ops::run_zahir_from_stream(
            path_rx,
            &ublx_opts_for_zahir,
            zahir_ops::ZahirOutputSink::Channel(output_tx),
        )
    });
    let on_entry = |e: &nefax_ops::NefaxEntry| {
        if e.size > 0 && zahir_ops::needs_zahir(prior_nefax.as_ref(), &e.path, e.mtime_ns) {
            let abs = dir_to_ublx_abs.join(&e.path).to_string_lossy().into_owned();
            let _ = path_tx.send(abs);
        }
    };
    let (nefax, diff) = match nefax_ops::run_nefaxer(
        dir_to_ublx,
        ublx_opts,
        prior_nefax.as_ref(),
        Some(on_entry),
    ) {
        Ok(result) => result,
        Err(e) => {
            drop(path_tx);
            let _ = zahir_handle.join();
            on_nefax_error(dir_to_ublx, &e);
        }
    };

    drop(path_tx);
    debug!("indexed {} paths (streaming)", nefax.len());
    if let Err(e) = db_ops::write_snapshot_to_db_streaming(
        dir_to_ublx,
        &nefax,
        &diff,
        &ublx_opts.to_ublx_settings(),
        output_rx,
        &prior_zahir_json,
    ) {
        error!("failed to write snapshot: {}", e);
        std::process::exit(1);
    }

    let zahir_result = match zahir_handle.join() {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            error!("zahir (stream) failed: {}", e);
            std::process::exit(1);
        }
        Err(_) => {
            error!("zahir thread panicked");
            std::process::exit(1);
        }
    };
    if let Err(e) = error_writer::write_zahir_failures_to_log(dir_to_ublx, &zahir_result) {
        error!("failed to write zahir failures to log: {}", e);
    }
    Ok(())
}
