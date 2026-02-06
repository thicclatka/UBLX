use log::{debug, error};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::config::UblxOpts;
use crate::engine::db_ops;
use crate::handlers::nefax_ops;
use crate::handlers::zahir_ops;
use crate::utils::{canonicalize_dir_to_ublx, error_writer};

pub fn run_sequential(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
) -> io::Result<()> {
    // Zahir opens paths with File::open(path) so paths must be absolute (cwd-independent).
    let dir_to_ublx_abs = dir_to_ublx
        .canonicalize()
        .unwrap_or_else(|_| dir_to_ublx.to_path_buf());
    let entry_callback: Option<fn(&nefax_ops::NefaxEntry)> = None;
    match nefax_ops::run_nefaxer(dir_to_ublx, ublx_opts, prior_nefax.as_ref(), entry_callback) {
        Ok((nefax, diff)) => {
            debug!(
                "indexed {} paths (added: {}, removed: {}, modified: {})",
                nefax.len(),
                diff.added.len(),
                diff.removed.len(),
                diff.modified.len()
            );
            let path_list: Vec<PathBuf> = nefax
                .iter()
                .filter(|(_, meta)| meta.size > 0)
                .map(|(p, _)| dir_to_ublx_abs.join(p))
                .collect();
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
            ) {
                error!("failed to write snapshot: {}", e);
                std::process::exit(1);
            }
            Ok(())
        }
        Err(e) => {
            let _ = error_writer::write_nefax_error_to_log(dir_to_ublx, &e);
            error!("nefax failed: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_stream(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
) -> io::Result<()> {
    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
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
        if e.size > 0 {
            let abs = dir_to_ublx_abs.join(&e.path).to_string_lossy().into_owned();
            let _ = path_tx.send(abs);
        }
    };
    match nefax_ops::run_nefaxer(dir_to_ublx, ublx_opts, prior_nefax.as_ref(), Some(on_entry)) {
        Ok((nefax, diff)) => {
            drop(path_tx);
            debug!("indexed {} paths (streaming)", nefax.len());
            if let Err(e) = db_ops::write_snapshot_to_db_streaming(
                dir_to_ublx,
                &nefax,
                &diff,
                &ublx_opts.to_ublx_settings(),
                output_rx,
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
        }
        Err(e) => {
            drop(path_tx);
            let _ = zahir_handle.join();
            let _ = error_writer::write_nefax_error_to_log(dir_to_ublx, &e);
            error!("nefax failed: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}
