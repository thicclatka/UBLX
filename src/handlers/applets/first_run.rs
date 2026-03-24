//! First launch: no `.ublx` DB yet and no local `ublx.toml` — ask for `enable_enhance_all`, write `ublx.toml`, then run index.

use crate::config::{UblxOpts, UblxPaths, write_visible_enhance_only_toml};
use crate::handlers::{applets::settings, snapshot};
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::{InitialEnhancePromptState, UblxState};
use crate::ui::keymap::UblxAction;

pub fn handle_initial_prompt(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    action: UblxAction,
) -> bool {
    let Some(ref mut prompt) = state.initial_prompt else {
        return false;
    };
    match action {
        UblxAction::MoveDown => {
            prompt.selected_index = (prompt.selected_index + 1).min(1);
            true
        }
        UblxAction::MoveUp => {
            prompt.selected_index = prompt.selected_index.saturating_sub(1);
            true
        }
        UblxAction::SearchSubmit => {
            let enable = prompt.selected_index == 0;
            finish_prompt(state, params, ublx_opts, enable);
            true
        }
        UblxAction::Quit | UblxAction::SearchClear => {
            finish_prompt(state, params, ublx_opts, false);
            true
        }
        _ => true,
    }
}

fn finish_prompt(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    enable_enhance_all: bool,
) {
    state.initial_prompt = None;
    let paths = UblxPaths::new(params.dir_to_ublx);
    if let Err(e) = write_visible_enhance_only_toml(&paths, enable_enhance_all) {
        log::warn!("write ublx.toml: {e}");
    }
    state.config_written_by_us_at = Some(std::time::Instant::now());
    settings::apply_config_reload(params, ublx_opts, state, Option::<&str>::None);
    if params.defer_first_snapshot {
        snapshot::spawn_snapshot_from_dir_db(
            params.dir_to_ublx,
            params.db_path,
            params.snapshot_done_tx.as_ref(),
            params.bumper,
            Some(ublx_opts),
        );
        params.defer_first_snapshot = false;
    }
}

/// Called from [`crate::handlers::core::run_ublx`] when the directory had no DB and no local config on startup.
pub fn init_prompt_state(state: &mut UblxState) {
    state.initial_prompt = Some(InitialEnhancePromptState { selected_index: 0 });
}
