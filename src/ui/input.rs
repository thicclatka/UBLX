use crossterm::event::{self, Event};
use std::io;

use crate::config::UblxOpts;
use crate::handlers::{
    applets::theme_selector::{self, ThemeContext},
    reload,
    state_transitions::UblxActionContext,
};
use crate::layout::{
    event_loop::RunUblxParams,
    setup::{RightPaneContent, UblxState, ViewData},
};
use crate::ui::{
    consts::UI_CONSTANTS,
    keymap::{UblxAction, key_action_setup, search_consumes},
    lens, menu,
};

/// Which main tabs are available (Duplicates, Lenses). Used for key binding and mode cycle.
#[derive(Clone, Copy)]
pub struct MainTabFlags {
    pub has_duplicates: bool,
    pub has_lenses: bool,
}

pub fn handle_ublx_input(
    state: &mut UblxState,
    view: &ViewData,
    right: &RightPaneContent,
    theme_ctx: ThemeContext<'_>,
    tabs: MainTabFlags,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) -> io::Result<bool> {
    if !event::poll(std::time::Duration::from_millis(UI_CONSTANTS.input_poll_ms))? {
        return Ok(false);
    }
    let Event::Key(e) = event::read()? else {
        return Ok(false);
    };
    let has_search_filter = !state.search.query.is_empty();
    let result = key_action_setup(
        e,
        state.search.active,
        has_search_filter,
        state.last_key_for_double,
        tabs.has_duplicates,
        tabs.has_lenses,
    );
    state.last_key_for_double = result.last_key_for_double;
    let action = result.action;

    if lens::handle_lens_name_input(state, params, e) {
        return Ok(false);
    }
    if lens::handle_lens_rename_input(state, params, e) {
        return Ok(false);
    }
    if lens::handle_lens_delete_confirm(state, params, action) {
        return Ok(false);
    }
    if menu::handle_space_menu(state, view, params, action) {
        return Ok(false);
    }
    if lens::handle_lens_menu(state, params, action) {
        return Ok(false);
    }

    if state.theme.selector_visible {
        theme_selector::handle_key(state, params, theme_ctx, action);
        return Ok(false);
    }
    if state.help_visible {
        state.help_visible = false;
        return Ok(false);
    }
    if menu::handle_open_menu(state, params, ublx_opts, action) {
        return Ok(false);
    }
    if menu::try_open_open_menu(state, right, action) {
        return Ok(false);
    }
    if lens::try_open_lens_menu(state, right, action) {
        return Ok(false);
    }
    if menu::try_open_space_menu(state, view, right, action) {
        return Ok(false);
    }

    if matches!(action, UblxAction::ThemeSelector) {
        theme_selector::open(state, theme_ctx);
        return Ok(false);
    }
    if matches!(action, UblxAction::ReloadConfig) {
        reload::apply_config_reload(
            params,
            ublx_opts,
            state,
            Some(crate::ui::UI_STRINGS.config_reloaded),
        );
        return Ok(false);
    }
    if matches!(action, UblxAction::SearchClear) {
        state.search.query.clear();
        state.search.active = false;
        return Ok(false);
    }
    if state.search.active {
        match action {
            UblxAction::SearchSubmit => state.search.active = false,
            UblxAction::SearchBackspace => {
                state.search.query.pop();
            }
            UblxAction::SearchChar(c) => state.search.query.push(c),
            _ => {}
        }
        if search_consumes(action) {
            return Ok(false);
        }
    }
    let ctx = UblxActionContext::new(view, right);
    Ok(ctx.apply_action_to_state(state, action, tabs.has_duplicates, tabs.has_lenses))
}
