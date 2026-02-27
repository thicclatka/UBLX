use crossterm::event::{self, Event};
use std::io;
use std::path::Path;

use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths, write_local_theme};
use crate::handlers::{core, open, reload, state_transitions::UblxActionContext};
use crate::layout::{
    event_loop::RunUblxParams,
    setup::{RightPaneContent, UblxState, ViewData},
    themes,
};
use crate::ui::{
    consts::{UI_CONSTANTS, UI_STRINGS},
    keymap::{UblxAction, key_action_setup, search_consumes},
};
use crate::utils::{format::clamp_selection, notifications::show_toast_slot};

/// Theme context for theme selector: (dir for local config, current theme name for preview/revert).
pub type ThemeContext<'a> = Option<(&'a Path, Option<&'a str>)>;

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
        let opts = themes::theme_options();
        let n = opts.len();
        match action {
            UblxAction::Quit | UblxAction::SearchClear => {
                state.theme_override = state.theme_before_selector.clone();
                state.theme_selector_visible = false;
            }
            UblxAction::MoveDown => {
                state.theme_selector_index = clamp_selection(state.theme_selector_index + 1, n);
            }
            UblxAction::MoveUp => {
                state.theme_selector_index =
                    clamp_selection(state.theme_selector_index.saturating_sub(1), n);
            }
            UblxAction::SearchSubmit => {
                let display_name = opts[state.theme_selector_index].display_name;
                if let Some((dir, _)) = theme_ctx {
                    write_local_theme(&UblxPaths::new(dir), display_name);
                    state.config_written_by_us_at = Some(std::time::Instant::now());
                }
                if let Some(b) = params.bumper {
                    b.push_with_operation(
                        log::Level::Info,
                        format!("Changed theme to {}", display_name),
                        Some(OPERATION_NAME.theme_selector()),
                    );
                    show_toast_slot(
                        &mut state.toast_slots,
                        b,
                        Some(OPERATION_NAME.theme_selector()),
                        &mut state.toast_consumed_per_operation,
                    );
                }
                state.theme_override = Some(display_name.to_string());
                state.theme_selector_visible = false;
            }
            _ => {}
        }
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
                state.open_menu_selected_index =
                    (state.open_menu_selected_index + 1).min(1);
            }
            UblxAction::MoveUp => {
                state.open_menu_selected_index =
                    state.open_menu_selected_index.saturating_sub(1);
            }
            UblxAction::SearchSubmit => {
                if let Some(ref rel_path) = state.open_menu_path {
                    let full_path = params.dir_to_ublx.join(rel_path);
                    if state.open_menu_selected_index == 0 {
                        if let Some(ed) = open::editor_for_open(ublx_opts.editor_path.as_deref())
                        {
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
        let current = theme_ctx
            .and_then(|(_, t)| t)
            .or(state.theme_override.as_deref());
        state.theme_before_selector = current.map(String::from);
        state.theme_selector_index = themes::theme_options()
            .iter()
            .position(|o| current == Some(o.display_name))
            .unwrap_or(0);
        state.theme_selector_visible = true;
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
