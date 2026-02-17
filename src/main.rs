mod macros;
mod_priv!(config, engine, handlers, layout, render, ui, utils);

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use log::debug;

use config::{TOAST_CONFIG, UblxOpts, UblxPaths};
use engine::db_ops;
use handlers::{core, nefax_ops};
use utils::notifications::{self, BumperBuffer};
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
    /// Dev mode: tui-logger drain + move_events, trace-level default filter
    #[arg(long = "dev")]
    dev: bool,
}

fn main() {
    let args = Args::parse();
    let start_time = args.test.then(Instant::now);

    let test_mode = args.test;
    let bumper = if test_mode {
        build_logger_test_mode_no_tui();
        None
    } else {
        let b = BumperBuffer::new(TOAST_CONFIG.bumper_cap_for(args.dev));
        notifications::init_logging(b.clone(), args.dev);
        Some(b)
    };

    let dir_to_ublx = validate_dir(&args.dir_to_ublx);
    debug!("indexing directory: {}", dir_to_ublx.display());

    let db_path = fatal!(
        db_ops::ensure_ublx_and_db(&dir_to_ublx),
        "failed to set up .ublx db: {}"
    );
    debug!("db: {}", db_path.display());

    // Load prior Nefax from DB or exit if error
    let prior_nefax = nefax_ops::load_prior_nefax_or_exit(&dir_to_ublx, &db_path);

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

    fatal!(
        core::run_app(core::RunAppParams {
            test_mode,
            dir_to_ublx: &dir_to_ublx,
            db_path: &db_path,
            ublx_opts: &ublx_opts,
            prior_nefax: &prior_nefax,
            bumper: bumper.as_ref(),
            dev: args.dev,
            start_time,
        }),
        "{}"
    );
}
