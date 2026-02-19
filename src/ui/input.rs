use crossterm::event::{self, Event};
use std::io;
use std::path::Path;
use std::time::Instant;

use crate::config::{write_local_theme, UblxPaths, OPERATION_NAME, TOAST_CONFIG};
use crate::layout::setup::{RightPaneContent, UblxActionContext, UblxState, ViewData};
use crate::layout::themes;
use crate::ui::keymap::{key_action_setup, search_consumes, UblxAction};
use crate::utils::notifications::BumperBuffer;

/// Theme context for theme selector: (dir for local config, current theme name for preview/revert).
pub type ThemeContext<'a> = Option<(&'a Path, Option<&'a str>)>;

pub fn handle_ublx_input(
    state: &mut UblxState,
    view: &ViewData,
    right: &RightPaneContent,
    theme_ctx: ThemeContext<'_>,
    bumper: Option<&BumperBuffer>,
) -> io::Result<bool> {
    if !event::poll(std::time::Duration::from_millis(100))? {
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
                state.theme_selector_index =
                    (state.theme_selector_index + 1).min(n.saturating_sub(1));
            }
            UblxAction::MoveUp => {
                state.theme_selector_index = state.theme_selector_index.saturating_sub(1);
            }
            UblxAction::SearchSubmit => {
                let display_name = opts[state.theme_selector_index].display_name;
                if let Some((dir, _)) = theme_ctx {
                    write_local_theme(&UblxPaths::new(dir), display_name);
                }
                if let Some(b) = bumper {
                    b.push_with_operation(
                        log::Level::Info,
                        format!("Changed theme to {}", display_name),
                        Some(OPERATION_NAME.theme_selector()),
                    );
                    state.toast_visible_until = Some(Instant::now() + TOAST_CONFIG.duration);
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
    Ok(ctx.apply_action_to_state(state, action))
}
