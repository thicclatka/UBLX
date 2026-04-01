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

#[derive(Parser)]
#[command(
    name = "ublx",
    about = "UBLX is a TUI to index once, enrich with metadata, and browse a flat snapshot in a 3-pane layout with multiple modes."
)]
struct Args {
    /// Directory to index
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

/// Headless indexing flag
#[derive(Parser)]
struct HeadlessCli {
    /// Headless snapshot. Writes a local config file when this dir has none.
    #[arg(long = "snapshot-only", short = 's')]
    snapshot_only: bool,
    /// With `--snapshot-only`: set `enable_enhance_all = true` in new local config and use it for this run.
    #[arg(long = "enhance-all", short = 'e')]
    enhance_all: bool,
    /// Same as `--snapshot-only --enhance-all`.
    #[arg(long = "full-snapshot", short = 'f')]
    full_snapshot: bool,
    /// Headless: write each Zahir JSON to `ublx-export/` as flat `{path}.json` files. Recommended to run with "--full-snapshot" to get most complete & recent results. Adjust enhance policy in config to fine-tune which paths get ZahirScan.
    #[arg(long = "export", short = 'x')]
    export_zahir: bool,
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
    let headless_snapshot = h.snapshot_only || h.full_snapshot;
    let headless_export = h.export_zahir;
    let headless = headless_snapshot || headless_export;

    utils::exit_if_enhance_all_without_headless(h.enhance_all, headless_snapshot);

    let start_time = headless_snapshot.then(Instant::now);

    let bumper = if headless {
        utils::build_logger_snapshot_only_no_tui();
        None
    } else {
        let b = utils::BumperBuffer::new(TOAST_CONFIG.bumper_cap_for(args.dev));
        utils::init_logging(b.clone(), args.dev);
        Some(b)
    };

    let local_enhance_all = h.enhance_all || h.full_snapshot;
    if h.full_snapshot && h.enhance_all {
        debug!("Full snapshot with --enhance-all is redundant; use --full-snapshot (-f) alone.");
    }

    let dir_to_ublx = utils::validate_dir(&args.dir_to_ublx);
    debug!("indexing directory: {}", dir_to_ublx.display());
    let had_any_cached_db_before_this_root = has_any_cached_ublx_db();

    let paths = UblxPaths::new(&dir_to_ublx);
    let had_index_db_before_ensure = paths.db().exists();

    let db_path = fatal!(
        db_ops::ensure_ublx_and_db(&dir_to_ublx),
        "failed to set up ublx db: {}"
    );
    debug!("db: {}", db_path.display());

    let initial_prompt = should_show_initial_prompt(headless_snapshot, had_index_db_before_ensure);
    debug!(
        "initial_prompt={initial_prompt} (had_index_db_before_ensure={had_index_db_before_ensure})"
    );
    debug!("cached ublx roots seen before startup: {had_any_cached_db_before_this_root}");

    // Prior Nefax + settings + (TUI only) snapshot/lens preload in one DB pass (`db_ops::load_tui_start_data`).
    let (prior_nefax_owned, tui_start, cached_settings) = if headless_snapshot {
        (
            load_prior_nefax_or_exit(&dir_to_ublx, &db_path),
            None,
            db_ops::load_settings_from_db(&db_path).ok().flatten(),
        )
    } else if headless_export {
        (
            None,
            None,
            db_ops::load_settings_from_db(&db_path).ok().flatten(),
        )
    } else {
        let load = fatal!(
            db_ops::load_tui_start_data(&db_path),
            "failed to load snapshot: {}"
        );
        let (prior, preload, cached) = load.split_for_app();
        (prior, Some(preload), cached)
    };
    if cached_settings.is_some() {
        debug!("using cached settings from ublx db (skipping disk check)");
    }

    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let for_dir_config = ublx::config::UblxOptsForDirExtras {
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
    if headless_snapshot && paths.toml_path().is_none() {
        ublx_opts.enable_enhance_all = local_enhance_all;
    }
    debug!("UBLX CONFIG: {ublx_opts:#?}");

    let mut run_params = handlers::RunAppParams {
        snapshot_only: headless_snapshot,
        export_only: headless_export,
        dir_to_ublx: &dir_to_ublx,
        db_path: &db_path,
        ublx_opts: &mut ublx_opts,
        prior_nefax: prior_nefax_owned.as_ref(),
        tui_start,
        bumper: bumper.as_ref(),
        dev: args.dev,
        start_time,
        initial_prompt,
    };
    fatal!(handlers::run_app(&mut run_params), "{}");

    if headless_snapshot && paths.toml_path().is_none() {
        fatal!(
            write_local_enhance_only_toml(&paths, local_enhance_all),
            "failed to write local config: {}"
        );
    }
}
