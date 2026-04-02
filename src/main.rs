use std::path::Path;
use std::time::Instant;

use clap::Parser;
use log::debug;

use ublx::cli_parser;
use ublx::config;
use ublx::engine::db_ops;
use ublx::handlers;
use ublx::integrations::{NefaxResult, load_prior_nefax_or_exit};
use ublx::themes;
use ublx::utils;

fn setup_bumper(headless_status: bool, dev: bool) -> Option<utils::BumperBuffer> {
    if headless_status {
        utils::build_logger_snapshot_only_no_tui();
        None
    } else {
        let b = utils::BumperBuffer::new(config::TOAST_CONFIG.bumper_cap_for(dev));
        utils::init_logging(b.clone(), dev);
        Some(b)
    }
}

fn load_startup_db_data(
    headless_mode_flags: cli_parser::HeadlessModeFlags,
    dir_to_ublx_ref: &Path,
    db_path_ref: &Path,
) -> (
    Option<NefaxResult>,
    Option<db_ops::TuiStartPreload>,
    Option<config::UblxSettings>,
) {
    let (prior_nefax_owned, tui_start, cached_settings) = if headless_mode_flags.snapshot.is_some()
    {
        (
            load_prior_nefax_or_exit(dir_to_ublx_ref, db_path_ref),
            None,
            db_ops::load_settings_from_db(db_path_ref).ok().flatten(),
        )
    } else if headless_mode_flags.export {
        (
            None,
            None,
            db_ops::load_settings_from_db(db_path_ref).ok().flatten(),
        )
    } else {
        let load = utils::fatal_error_handler(
            db_ops::load_tui_start_data(db_path_ref),
            "failed to load snapshot: {}",
        );
        let (prior, preload, cached) = load.split_for_app();
        (prior, Some(preload), cached)
    };
    (prior_nefax_owned, tui_start, cached_settings)
}

fn main() {
    let args = cli_parser::Args::parse();
    if args.themes {
        cli_parser::print_available_themes();
        return;
    }

    let headless_mode_flags = cli_parser::headless_handler(&args.headless);

    let start_time = headless_mode_flags.snapshot.is_some().then(Instant::now);

    let bumper = setup_bumper(headless_mode_flags.is_headless(), args.dev);

    let local_enhance_all = headless_mode_flags.determine_enhance_all();

    let dir_to_ublx = utils::validate_dir(&args.dir_to_ublx);

    // Install panic hook with ublx log
    utils::install_panic_hook_with_ublx_log(&dir_to_ublx);

    debug!("indexing directory: {}", dir_to_ublx.display());

    let paths = config::UblxPaths::new(&dir_to_ublx);
    let had_index_db_before_ensure = paths.db().exists();
    let had_any_cached_db_before_this_root = config::has_any_cached_ublx_db();

    let db_path = utils::fatal_error_handler(
        db_ops::ensure_ublx_and_db(&dir_to_ublx),
        "failed to set up ublx db: {}",
    );
    debug!("db: {}", db_path.display());

    let initial_prompt = config::should_show_initial_prompt(
        headless_mode_flags.snapshot.is_some(),
        had_index_db_before_ensure,
        had_any_cached_db_before_this_root,
    );

    // Prior Nefax + settings + (TUI only) snapshot/lens preload in one DB pass (`db_ops::load_tui_start_data`).
    let (prior_nefax_owned, tui_start, cached_settings) =
        load_startup_db_data(headless_mode_flags, &dir_to_ublx, &db_path);

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
    let mut ublx_opts = config::UblxOpts::for_dir(
        &dir_to_ublx,
        &paths,
        None,
        None,
        None,
        cached_settings.as_ref(),
        &for_dir_config,
    );

    if headless_mode_flags.snapshot.is_some() && paths.toml_path().is_none() {
        ublx_opts.enable_enhance_all = local_enhance_all;
    }

    debug!("UBLX CONFIG: {ublx_opts:#?}");

    let mut run_params = handlers::RunAppParams {
        headless: &handlers::RunAppParamsHeadless {
            snapshot_only: headless_mode_flags.snapshot.is_some(),
            export_only: headless_mode_flags.export,
        },
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
    utils::fatal_error_handler(handlers::run_app(&mut run_params), "{}");

    if headless_mode_flags.snapshot.is_some() && paths.toml_path().is_none() {
        utils::fatal_error_handler(
            config::write_local_enhance_only_toml(&paths, local_enhance_all),
            "failed to write local config: {}",
        );
    }
}
