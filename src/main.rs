mod macros;
mod_priv!(config, engine, handlers, utils);

use std::path::PathBuf;
use std::sync::mpsc;

use clap::Parser;
use log::{debug, error, info};

use config::{UblxOpts, UblxPaths};
use engine::*;
use handlers::{nefax_ops, stream_seq, zahir_ops};
use utils::*;

#[derive(Parser)]
#[command(name = "ublx")]
struct Args {
    /// Directory to index (default: current directory)
    #[arg(default_value = ".")]
    dir: PathBuf,
    /// Do a test run, no TUI, write snapshot to .ublx
    #[arg(short = 't', long = "test")]
    test: bool,
}

fn main() {
    let args = Args::parse();
    let dir = validate_dir(&args.dir);

    let test_mode = args.test;
    let (bumper, dev) = if test_mode {
        build_logger_test_mode_no_tui();
        (None, false)
    } else {
        let dev = notifications::is_dev_mode();
        let bumper = notifications::BumperBuffer::new(notifications::DEFAULT_BUMPER_CAP);
        notifications::init_logging(bumper.clone(), dev);
        (Some(bumper), dev)
    };

    debug!("indexing directory: {}", dir.display());

    let db_path = match db_ops::ensure_ublx_and_db(&dir) {
        Ok(p) => p,
        Err(e) => {
            error!("failed to set up .ublx db: {}", e);
            std::process::exit(1);
        }
    };
    debug!("db: {}", db_path.display());

    let prior_nefax = match db_ops::load_nefax_from_db(&dir, &db_path) {
        Ok(Some(nefax)) => {
            debug!("loaded {} paths from snapshot", nefax.len());
            Some(nefax)
        }
        Ok(None) => None,
        Err(e) => {
            error!("failed to load snapshot: {}", e);
            std::process::exit(1);
        }
    };

    let paths = UblxPaths::new(&dir);
    let ublx_opts = UblxOpts::for_dir(&dir, &paths, None, None, None);
    debug!("UBLX CONFIG: {:?}", ublx_opts);
    let mode = stream_seq::RunMode::from_opts(&ublx_opts);

    match mode {
        stream_seq::RunMode::Sequential => {
            let entry_callback: Option<fn(&nefax_ops::NefaxEntry)> = None;
            match handlers::nefax_ops::run_nefaxer(
                &dir,
                &ublx_opts,
                prior_nefax.as_ref(),
                entry_callback,
            ) {
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
                        .map(|(p, _)| p.clone())
                        .collect();
                    let zahir_result = match zahir_ops::run_zahir_batch(&path_list, &ublx_opts) {
                        Ok(r) => r,
                        Err(e) => {
                            error!("zahir (sequential) failed: {}", e);
                            std::process::exit(1);
                        }
                    };
                    // Capture phase1_failed / phase2_failed for later use
                    let _phase1_errors = &zahir_result.phase1_failed;
                    let _phase2_errors = &zahir_result.phase2_failed;
                    if let Err(e) = db_ops::write_snapshot_to_db(&dir, &nefax, &zahir_result) {
                        error!("failed to write snapshot: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    error!("nefax failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        stream_seq::RunMode::Stream => {
            let ublx_opts_for_zahir = ublx_opts.clone();
            let (tx, rx) = mpsc::channel();
            let zahir_handle = std::thread::spawn(move || {
                zahir_ops::run_zahir_from_stream(rx, &ublx_opts_for_zahir)
            });
            let on_entry = |e: &nefax_ops::NefaxEntry| {
                if e.size > 0 {
                    let _ = tx.send(e.path.to_string_lossy().into_owned());
                }
            };
            match handlers::nefax_ops::run_nefaxer(
                &dir,
                &ublx_opts,
                prior_nefax.as_ref(),
                Some(on_entry),
            ) {
                Ok((nefax, _diff)) => {
                    drop(tx);
                    info!("indexed {} paths (streaming)", nefax.len());
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
                    let _phase1_errors = &zahir_result.phase1_failed;
                    let _phase2_errors = &zahir_result.phase2_failed;
                    if let Err(e) = db_ops::write_snapshot_to_db(&dir, &nefax, &zahir_result) {
                        error!("failed to write snapshot: {}", e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    drop(tx);
                    let _ = zahir_handle.join();
                    error!("nefax failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    if let Err(e) = db_ops::post_ublx_run_cleanup(&dir) {
        error!("failed to cleanup: {}", e);
        std::process::exit(1);
    }

    if test_mode {
        return;
    }
    if let Some(b) = bumper
        && let Err(e) = orchestrator::run_ublx(&b, dev)
    {
        error!("TUI error: {}", e);
        std::process::exit(1);
    }
}
