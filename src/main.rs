use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use log::debug;

use ublx::config::{
    TOAST_CONFIG, UblxOpts, UblxPaths, has_any_cached_ublx_db, should_show_initial_prompt,
    write_local_enhance_only_toml,
};
use ublx::engine::db_ops;
use ublx::fatal;
use ublx::handlers;
use ublx::integrations::load_prior_nefax_or_exit;
use ublx::themes;
use ublx::utils;

/// Headless indexing flags (kept in one struct so the top-level [`Args`] stays under clippy’s bool limit).
#[derive(Parser)]
struct HeadlessCli {
    /// Headless snapshot then exit (no TUI). Writes `.ublx.toml` when this dir has no local config yet.
    #[arg(long = "snapshot-only", short = 's')]
    snapshot_only: bool,
    /// With `--snapshot-only`: set `enable_enhance_all = true` in new local config and use it for this run. Incompatible with `--full-snapshot`.
    #[arg(long = "enhance-all", short = 'e', conflicts_with = "full_snapshot")]
    enhance_all: bool,
    /// Same as `--snapshot-only --enhance-all`.
    #[arg(long = "full-snapshot", short = 'f')]
    full_snapshot: bool,
}

#[derive(Parser)]
#[command(name = "ublx")]
struct Args {
    /// Directory to index (default: current directory)
    #[arg(value_name = "DIR", default_value = ".")]
    dir_to_ublx: PathBuf,
    #[command(flatten)]
    headless: HeadlessCli,
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

    let h = &args.headless;
    let headless = h.snapshot_only || h.full_snapshot;
    utils::exit_if_enhance_all_without_headless(h.enhance_all, headless);
    let local_enhance_all = h.full_snapshot || h.enhance_all;

    let start_time = headless.then(Instant::now);

    let bumper = if headless {
        utils::build_logger_snapshot_only_no_tui();
        None
    } else {
        let b = utils::BumperBuffer::new(TOAST_CONFIG.bumper_cap_for(args.dev));
        utils::init_logging(b.clone(), args.dev);
        Some(b)
    };

    let dir_to_ublx = utils::validate_dir(&args.dir_to_ublx);
    debug!("indexing directory: {}", dir_to_ublx.display());
    let had_any_cached_db_before_this_root = has_any_cached_ublx_db();

    let paths = UblxPaths::new(&dir_to_ublx);
    let had_ubli_db = paths.db().exists();

    let db_path = fatal!(
        db_ops::ensure_ublx_and_db(&dir_to_ublx),
        "failed to set up ublx db: {}"
    );
    debug!("db: {}", db_path.display());

    let initial_prompt = should_show_initial_prompt(headless, had_ubli_db);
    debug!("initial_prompt={initial_prompt} (had_ubli_db={had_ubli_db})");
    debug!("cached ublx roots seen before startup: {had_any_cached_db_before_this_root}");

    // Load prior Nefax from DB or exit if error
    let prior_nefax = load_prior_nefax_or_exit(&dir_to_ublx, &db_path);

    let cached_settings = db_ops::load_settings_from_db(&db_path).ok().flatten();
    if cached_settings.is_some() {
        debug!("using cached settings from ublx db (skipping disk check)");
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
    if headless && paths.toml_path().is_none() {
        ublx_opts.enable_enhance_all = local_enhance_all;
    }
    debug!("UBLX CONFIG: {ublx_opts:#?}");

    let mut run_params = handlers::RunAppParams {
        snapshot_only: headless,
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

    if headless && paths.toml_path().is_none() {
        fatal!(
            write_local_enhance_only_toml(&paths, local_enhance_all),
            "failed to write local config: {}"
        );
    }
}
