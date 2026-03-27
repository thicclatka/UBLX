//! First launch: empty snapshot and no local `ublx.toml` — root choice, optional prior-root pick, then `enable_enhance_all` prompt (unless disabled in global config).

use std::path::PathBuf;
use std::process;

use crate::app::RunUblxParams;
use crate::config::{
    UblxOpts, UblxPaths, prior_indexed_roots_recent, record_prior_root_selected,
    remember_indexed_root_path, write_visible_enhance_only_toml,
};
use crate::handlers::{applets::settings, core, snapshot};
use crate::layout::setup::{StartupPromptPhase, StartupPromptState, UblxState};
use crate::ui::keymap::UblxAction;

fn relaunch_ublx(dir_ref: &PathBuf) -> ! {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ublx"));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = process::Command::new(&exe).arg(dir_ref).exec();
        eprintln!("ublx: failed to relaunch: {err}");
        process::exit(126);
    }
    #[cfg(not(unix))]
    {
        let status = process::Command::new(&exe).arg(dir_ref).status();
        process::exit(status.map(|s| s.code().unwrap_or(1)).unwrap_or(1));
    }
}

pub fn handle_startup_prompt(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    action: UblxAction,
) -> bool {
    let Some(ref mut sp) = state_mut.startup_prompt else {
        return false;
    };
    match &mut sp.phase {
        StartupPromptPhase::RootChoice {
            selected_index,
            roots,
        } => match action {
            UblxAction::MoveDown => {
                let max = roots.len();
                *selected_index = (*selected_index + 1).min(max);
                true
            }
            UblxAction::MoveUp => {
                *selected_index = selected_index.saturating_sub(1);
                true
            }
            UblxAction::SearchSubmit => {
                if *selected_index == 0 {
                    index_this_directory(state_mut, params_mut, ublx_opts_mut);
                } else if let Some(dir) = roots.get(*selected_index - 1).cloned() {
                    let _ = record_prior_root_selected(&dir);
                    core::restore_terminal();
                    relaunch_ublx(&dir);
                }
                true
            }
            UblxAction::Quit | UblxAction::SearchClear => {
                core::restore_terminal();
                process::exit(0);
            }
            _ => true,
        },
        StartupPromptPhase::Enhance { selected_index } => match action {
            UblxAction::MoveDown => {
                *selected_index = (*selected_index + 1).min(1);
                true
            }
            UblxAction::MoveUp => {
                *selected_index = selected_index.saturating_sub(1);
                true
            }
            UblxAction::SearchSubmit => {
                let enable = *selected_index == 0;
                finish_enhance_flow(state_mut, params_mut, ublx_opts_mut, enable);
                true
            }
            UblxAction::Quit | UblxAction::SearchClear => {
                finish_enhance_flow(state_mut, params_mut, ublx_opts_mut, false);
                true
            }
            _ => true,
        },
    }
}

fn index_this_directory(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
) {
    if ublx_opts_mut.ask_enhance_on_new_root {
        if let Some(sp) = state_mut.startup_prompt.as_mut() {
            sp.phase = StartupPromptPhase::Enhance { selected_index: 0 };
        }
    } else {
        finish_enhance_flow(
            state_mut,
            params_mut,
            ublx_opts_mut,
            ublx_opts_mut.enable_enhance_all,
        );
    }
}

fn finish_enhance_flow(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    enable_enhance_all: bool,
) {
    state_mut.startup_prompt = None;
    let _ = remember_indexed_root_path(params_mut.dir_to_ublx);
    let paths = UblxPaths::new(params_mut.dir_to_ublx);
    if let Err(e) = write_visible_enhance_only_toml(&paths, enable_enhance_all) {
        log::warn!("write ublx.toml: {e}");
    }
    state_mut.config_written_by_us_at = Some(std::time::Instant::now());
    settings::apply_config_reload(params_mut, ublx_opts_mut, state_mut, Option::<&str>::None);
    if params_mut.startup.defer_first_snapshot {
        snapshot::spawn_snapshot_from_dir_db(
            params_mut.dir_to_ublx,
            params_mut.db_path,
            params_mut.snapshot_done_tx.as_ref(),
            params_mut.bumper,
            Some(ublx_opts_mut),
        );
        params_mut.startup.defer_first_snapshot = false;
    }
}

/// Called from [`crate::handlers::core::run_ublx`] when the directory had an empty snapshot and no local config on startup.
pub fn init_prompt_state(state_mut: &mut UblxState, current_ref: &std::path::Path) {
    let roots = prior_indexed_roots_recent(current_ref, 5);
    state_mut.startup_prompt = Some(StartupPromptState {
        phase: StartupPromptPhase::RootChoice {
            selected_index: 0,
            roots,
        },
    });
}
