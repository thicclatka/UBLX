mod macros;
mod_priv!(config, engine, handlers, layout, ui, utils);

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use log::{debug, error};

use config::{RunMode, UblxOpts, UblxPaths};
use engine::*;
use layout::event_loop::run_ublx;
use utils::*;

#[derive(Parser)]
#[command(name = "ublx")]
struct Args {
    /// Directory to index (default: current directory)
    #[arg(value_name = "DIR", default_value = ".")]
    dir_to_ublx: PathBuf,
    /// Do a test run, no TUI, write snapshot to .ublx
    #[arg(short = 't', long = "test")]
    test: bool,
}

fn main() {
    let args = Args::parse();
    let start_time = args.test.then(Instant::now);

    let dir_to_ublx = validate_dir(&args.dir_to_ublx);

    let test_mode = args.test;
    build_logger_test_mode_no_tui();

    debug!("indexing directory: {}", dir_to_ublx.display());

    let db_path = match db_ops::ensure_ublx_and_db(&dir_to_ublx) {
        Ok(p) => p,
        Err(e) => {
            error!("failed to set up .ublx db: {}", e);
            std::process::exit(1);
        }
    };
    debug!("db: {}", db_path.display());

    let prior_nefax = match db_ops::load_nefax_from_db(&dir_to_ublx, &db_path) {
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

    let cached_settings = db_ops::load_settings_from_db(&db_path).ok().flatten();
    if cached_settings.is_some() {
        debug!("using cached settings from .ublx (skipping disk check)");
    }

    let paths = UblxPaths::new(&dir_to_ublx);
    let ublx_opts = UblxOpts::for_dir(
        &dir_to_ublx,
        &paths,
        None,
        None,
        None,
        cached_settings.as_ref(),
    );
    debug!("UBLX CONFIG: {:#?}", ublx_opts);
    let mode = RunMode::from_opts(&ublx_opts);

    match mode {
        RunMode::Sequential => {
            if let Err(e) = orchestrator::run_sequential(&dir_to_ublx, &ublx_opts, &prior_nefax) {
                error!("sequential mode failed: {}", e);
                if test_mode {
                    std::process::exit(1);
                }
            }
        }
        RunMode::Stream => {
            if let Err(e) = orchestrator::run_stream(&dir_to_ublx, &ublx_opts, &prior_nefax) {
                error!("stream mode failed: {}", e);
                if test_mode {
                    std::process::exit(1);
                }
            }
        }
    }

    if let Err(e) = db_ops::UblxCleanup::new(&dir_to_ublx).post_run_cleanup() {
        error!("failed to cleanup: {}", e);
        if test_mode {
            std::process::exit(1);
        }
    }

    if test_mode {
        let duration = start_time.unwrap().elapsed();
        debug!(
            "UBLX test completed in {:.4?} seconds",
            duration.as_secs_f64()
        );
        return;
    }

    if let Err(e) = run_ublx(&db_path, &dir_to_ublx) {
        error!("TUI error: {}", e);
        eprintln!("ublx: TUI error: {}", e);
        std::process::exit(1);
    }
}
