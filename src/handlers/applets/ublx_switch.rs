//! Switch indexed project: list recents-backed roots with a DB, then switch in-process to the chosen root.

use crate::app::RunUblxParams;
use crate::config::{all_indexed_roots_alphabetical, record_prior_root_selected};
use crate::layout::setup::{UblxState, UblxSwitchPickerState};
use crate::ui::UblxAction;

pub fn open(state_mut: &mut UblxState, params: &RunUblxParams<'_>) {
    let roots = all_indexed_roots_alphabetical();
    let current = params.dir_to_ublx.as_path();
    let current_canon = current
        .canonicalize()
        .unwrap_or_else(|_| current.to_path_buf());
    let sel = roots
        .iter()
        .position(|p| {
            let c = p.canonicalize().unwrap_or_else(|_| p.clone());
            c == current_canon
        })
        .unwrap_or(0);
    let max = roots.len().saturating_sub(1);
    state_mut.chrome.ublx_switch = UblxSwitchPickerState {
        visible: true,
        selected_index: sel.min(max),
        roots,
    };
}

pub fn handle_key(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    action: UblxAction,
) {
    let sw = &mut state_mut.chrome.ublx_switch;
    let n = sw.roots.len();
    match action {
        UblxAction::Quit | UblxAction::SearchClear => {
            sw.visible = false;
        }
        UblxAction::MoveDown if n > 0 => {
            sw.selected_index = (sw.selected_index + 1).min(n - 1);
        }
        UblxAction::MoveUp if n > 0 => {
            sw.selected_index = sw.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            if n == 0 {
                sw.visible = false;
                return;
            }
            let dir = sw.roots[sw.selected_index].clone();
            let cur_c = params_mut
                .dir_to_ublx
                .canonicalize()
                .unwrap_or_else(|_| params_mut.dir_to_ublx.clone());
            let pick_c = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            if pick_c == cur_c {
                sw.visible = false;
                return;
            }
            let _ = record_prior_root_selected(&dir);
            sw.visible = false;
            state_mut.session.pending_switch_to = Some(dir);
        }
        _ => {}
    }
}
