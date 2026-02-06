use crossterm::event::{self, Event};

use std::io;

use crate::layout::setup::{RightPaneContent, UblxActionContext, UblxState, ViewData};
use crate::ui::keymap::{UblxAction, key_action_setup, search_consumes};

pub fn handle_ublx_input(
    state: &mut UblxState,
    view: &ViewData,
    right: &RightPaneContent,
) -> io::Result<bool> {
    if !event::poll(std::time::Duration::from_millis(100))? {
        return Ok(false);
    }
    let Event::Key(e) = event::read()? else {
        return Ok(false);
    };
    let has_search_filter = !state.search_query.is_empty();
    let action = key_action_setup(e, state.search_active, has_search_filter);
    if state.help_visible {
        state.help_visible = false;
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
