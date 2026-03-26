use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use log::debug;

use ublx::config::{TOAST_CONFIG, UblxOpts, UblxPaths};
use ublx::engine::db_ops;
use ublx::fatal;
use ublx::handlers;
use ublx::integrations::load_prior_nefax_or_exit;
use ublx::themes;
use ublx::utils::{build_logger_test_mode_no_tui, notifications, validate_dir};

#[derive(Parser)]
#[command(name = "ublx")]
struct Args {
    /// Directory to index (default: current directory)
    #[arg(value_name = "DIR", default_value = ".")]
    dir_to_ublx: PathBuf,
    /// Do a test run, no TUI, write snapshot to .ublx
    #[arg(short = 't', long = "test")]
    test: bool,
    /// Dev mode: tui-logger drain + `move_events` + trace-level default filter
    #[arg(long = "dev")]
    dev: bool,
    /// Print available themes grouped by appearance
    #[arg(long = "themes")]
    themes: bool,
}

fn print_available_themes() {
    for entry in themes::theme_selector_entries() {
        match entry {
            themes::SelectorEntry::Section(label) => {
                println!("{label}:");
            }
            themes::SelectorEntry::Item(theme) => {
                println!("  - {}", theme.name);
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    if args.themes {
        print_available_themes();
        return;
    }
    let start_time = args.test.then(Instant::now);

    let test_mode = args.test;
    let bumper = if test_mode {
        build_logger_test_mode_no_tui();
        None
    } else {
        let b = notifications::BumperBuffer::new(TOAST_CONFIG.bumper_cap_for(args.dev));
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

    let paths = UblxPaths::new(&dir_to_ublx);
    // After `ensure_ublx_and_db`, `.ublx` always exists — use empty `snapshot` + no local toml, not missing DB file.
    let initial_prompt =
        !test_mode && !db_ops::snapshot_has_any_row(&db_path) && paths.toml_path().is_none();

    // Load prior Nefax from DB or exit if error
    let prior_nefax = load_prior_nefax_or_exit(&dir_to_ublx, &db_path);

    let cached_settings = db_ops::load_settings_from_db(&db_path).ok().flatten();
    if cached_settings.is_some() {
        debug!("using cached settings from .ublx (skipping disk check)");
    }

    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let for_dir_config = ublx::config::ForDirConfig {
        valid_theme_names: &valid_themes,
        bumper: bumper.as_ref(),
    };
    let mut ublx_opts = UblxOpts::for_dir(
        &dir_to_ublx,
        &paths,
        None,
        None,
        None,
        cached_settings.as_ref(),
        &for_dir_config,
    );
    debug!("UBLX CONFIG: {ublx_opts:#?}");

    let mut run_params = handlers::RunAppParams {
        test_mode,
        dir_to_ublx: &dir_to_ublx,
        db_path: &db_path,
        ublx_opts: &mut ublx_opts,
        prior_nefax: prior_nefax.as_ref(),
        bumper: bumper.as_ref(),
        dev: args.dev,
        start_time,
        initial_prompt,
    };
    fatal!(handlers::run_app(&mut run_params), "{}");
}
