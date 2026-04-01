//! Open menu (from quick actions menu (spacebar)) — Terminal vs GUI.

use crate::app::RunUblxParams;
use crate::config::UblxOpts;
use crate::handlers::leave_terminal_for_editor;
use crate::layout::setup::UblxState;
use crate::modules;
use crate::ui::keymap::UblxAction;

fn open_menu_on_submit(
    state: &mut UblxState,
    params: &RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) {
    if let Some(rel_path) = state.open_menu.path.as_ref() {
        let full_path = params.dir_to_ublx.join(rel_path);
        let terminal_choice = state.open_menu.can_terminal && state.open_menu.selected_index == 0;
        if terminal_choice {
            if let Some(ed) = modules::opener::editor_for_open(ublx_opts.editor_path.as_deref()) {
                let _ = leave_terminal_for_editor();
                let _ = modules::opener::open_in_editor(&ed, &full_path);
                state.session.tick.refresh_terminal_after_editor = true;
            }
        } else {
            let _ = modules::opener::open_in_gui(&full_path);
        }
    }
    state.close_open_menu();
}

/// Handle action when Open menu (Terminal/GUI) is visible. Returns true if handled.
pub fn handle_open_menu(
    state: &mut UblxState,
    params: &RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    action: UblxAction,
) -> bool {
    if !state.open_menu.visible {
        return false;
    }
    match action {
        UblxAction::Quit | UblxAction::SearchClear => state.close_open_menu(),
        UblxAction::MoveDown => {
            let max_idx = usize::from(state.open_menu.can_terminal);
            state.open_menu.selected_index = (state.open_menu.selected_index + 1).min(max_idx);
        }
        UblxAction::MoveUp => {
            state.open_menu.selected_index = state.open_menu.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => open_menu_on_submit(state, params, ublx_opts),
        _ => {}
    }
    true
}
