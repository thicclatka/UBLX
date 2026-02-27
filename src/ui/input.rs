use crossterm::event::{self, Event};
use std::io;

use crate::config::UblxOpts;
use crate::handlers::{
    applets::theme_selector, core, open, reload, state_transitions::UblxActionContext,
};

/// Theme context for theme selector (dir for local config, current theme name). Re-exported from [crate::handlers::applets::theme_selector].
pub use crate::handlers::applets::theme_selector::ThemeContext;
use crate::layout::{
    event_loop::RunUblxParams,
    setup::{RightPaneContent, UblxState, ViewData},
};
use crate::ui::{
    consts::{UI_CONSTANTS, UI_STRINGS},
    keymap::{UblxAction, key_action_setup, search_consumes},
};

pub fn handle_ublx_input(
    state: &mut UblxState,
    view: &ViewData,
    right: &RightPaneContent,
    theme_ctx: ThemeContext<'_>,
    has_duplicates: bool,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) -> io::Result<bool> {
    if !event::poll(std::time::Duration::from_millis(UI_CONSTANTS.input_poll_ms))? {
        return Ok(false);
    }
    let Event::Key(e) = event::read()? else {
        return Ok(false);
    };
    let has_search_filter = !state.search_query.is_empty();
    let result = key_action_setup(
        e,
        state.search_active,
        has_search_filter,
        state.last_key_for_double,
        has_duplicates,
    );
    state.last_key_for_double = result.last_key_for_double;
    let action = result.action;

    if state.theme_selector_visible {
        theme_selector::handle_key(state, params, theme_ctx, action);
        return Ok(false);
    }

    if state.help_visible {
        state.help_visible = false;
        return Ok(false);
    }
    if state.open_menu_visible {
        match action {
            UblxAction::Quit | UblxAction::SearchClear => {
                state.open_menu_visible = false;
                state.open_menu_path = None;
            }
            UblxAction::MoveDown => {
                state.open_menu_selected_index = (state.open_menu_selected_index + 1).min(1);
            }
            UblxAction::MoveUp => {
                state.open_menu_selected_index = state.open_menu_selected_index.saturating_sub(1);
            }
            UblxAction::SearchSubmit => {
                if let Some(ref rel_path) = state.open_menu_path {
                    let full_path = params.dir_to_ublx.join(rel_path);
                    if state.open_menu_selected_index == 0 {
                        if let Some(ed) = open::editor_for_open(ublx_opts.editor_path.as_deref()) {
                            let _ = core::leave_terminal_for_editor();
                            let _ = open::open_in_editor(&ed, &full_path);
                            state.refresh_terminal_after_editor = true;
                        }
                    } else {
                        let _ = open::open_in_gui(&full_path);
                    }
                }
                state.open_menu_visible = false;
                state.open_menu_path = None;
            }
            _ => {}
        }
        return Ok(false);
    }
    if matches!(action, UblxAction::OpenMenu)
        && right.viewer_can_open
        && right.viewer_path.is_some()
    {
        state.open_menu_visible = true;
        state.open_menu_path = right.viewer_path.clone();
        state.open_menu_selected_index = 0;
        return Ok(false);
    }
    if matches!(action, UblxAction::ThemeSelector) {
        theme_selector::open(state, theme_ctx);
        return Ok(false);
    }
    if matches!(action, UblxAction::ReloadConfig) {
        reload::apply_config_reload(params, ublx_opts, state, Some(UI_STRINGS.config_reloaded));
        return Ok(false);
    }
    if matches!(action, UblxAction::SearchClear) {
        state.search_query.clear();
        state.search_active = false;
        return Ok(false);
    }
    if state.search_active {
        match action {
            UblxAction::SearchSubmit => state.search_active = false,
            UblxAction::SearchBackspace => {
                state.search_query.pop();
            }
            UblxAction::SearchChar(c) => state.search_query.push(c),
            _ => {}
        }
        if search_consumes(action) {
            return Ok(false);
        }
    }
    let ctx = UblxActionContext::new(view, right);
    Ok(ctx.apply_action_to_state(state, action, has_duplicates))
}
