//! Space menu → subtree `[[enhance_policy]]` (index-time batch Zahir only).

use crate::config::{EnhancePolicy, UblxOpts, UblxPaths, write_local_enhance_policy};
use crate::handlers::applets::settings::apply_config_reload;
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::UblxState;
use crate::ui::{UI_STRINGS, keymap::UblxAction, show_operation_toast};
use crate::utils::format::clamp_selection;

const CHOICES: usize = 2;

/// Handle keys while the enhance-policy chooser is open. Returns true when this modal is active (consumed or not).
pub fn handle_enhance_policy_menu(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    action: UblxAction,
) -> bool {
    if !state.enhance_policy_menu.visible {
        return false;
    }
    match action {
        UblxAction::Quit | UblxAction::SearchClear => state.close_enhance_policy_menu(),
        UblxAction::MoveDown => {
            state.enhance_policy_menu.selected_index =
                clamp_selection(state.enhance_policy_menu.selected_index + 1, CHOICES);
        }
        UblxAction::MoveUp => {
            state.enhance_policy_menu.selected_index = clamp_selection(
                state.enhance_policy_menu.selected_index.saturating_sub(1),
                CHOICES,
            );
        }
        UblxAction::SearchSubmit => {
            let Some(path) = state.enhance_policy_menu.path.clone() else {
                state.close_enhance_policy_menu();
                return true;
            };
            let policy = if state.enhance_policy_menu.selected_index == 0 {
                EnhancePolicy::Auto
            } else {
                EnhancePolicy::Manual
            };
            write_local_enhance_policy(&UblxPaths::new(params.dir_to_ublx), &path, policy);
            state.config_written_by_us_at = Some(std::time::Instant::now());
            apply_config_reload(params, ublx_opts, state, None::<&str>);
            let label = match policy {
                EnhancePolicy::Auto => UI_STRINGS.space.enhance_policy_always,
                EnhancePolicy::Manual => UI_STRINGS.space.enhance_policy_never,
            };
            let msg = if matches!(policy, EnhancePolicy::Auto) {
                if state.snapshot_bg.done_received {
                    state.snapshot_bg.requested = true;
                    format!("Enhance policy: {label} — updating index (Zahir for subtree)")
                } else {
                    state.snapshot_bg.defer_snapshot_after_current = true;
                    format!("Enhance policy: {label} — index will update after current run")
                }
            } else {
                format!("Enhance policy (index): {label} for subtree")
            };
            show_operation_toast(state, params, msg, "enhance-policy", log::Level::Info);
            state.close_enhance_policy_menu();
        }
        _ => {}
    }
    true
}
