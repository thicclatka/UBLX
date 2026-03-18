use crossterm::event::{self, Event, KeyEvent};
use std::io;

use crate::config::UblxOpts;
use crate::handlers::{applets, state_transitions};
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

/// Key event and resolved action passed into modal handlers (keeps [dispatch_modal_handlers] under clippy’s arg limit).
struct ModalInput {
    e: KeyEvent,
    action: UblxAction,
}

/// Run modal handlers in order; returns true if any handler consumed the event (caller should then return Ok(false)).
/// Keeps the main input function short and makes it easy to add new modals in one place.
///
/// Rust note: a single dispatch function like this is the usual pattern when each handler has different arguments.
/// A true "(guard, handler)" table would require either (1) a common context struct passed to every handler, or
/// (2) `Box<dyn FnOnce() -> bool>` and passing captured state into a runner—often not worth it until you have
/// many more handlers or need to register them dynamically.
fn dispatch_modal_handlers(
    state: &mut UblxState,
    view: &ViewData,
    right_content: &RightPaneContent,
    theme_ctx: applets::theme_selector::ThemeContext<'_>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    input: ModalInput,
) -> bool {
    let ModalInput { e, action } = input;
    // Table-style: each line is (guard / handler returns true) → we handled the event.
    if lens::handle_lens_name_input(state, params, e) {
        return true;
    }
    if lens::handle_lens_rename_input(state, params, e) {
        return true;
    }
    if lens::handle_lens_delete_confirm(state, params, action) {
        return true;
    }
    if menu::handle_space_menu(state, view, params, action) {
        return true;
    }
    if lens::handle_lens_menu(state, params, action) {
        return true;
    }
    if state.theme.selector_visible {
        applets::theme_selector::handle_key(state, params, theme_ctx, action);
        return true;
    }
    if state.help_visible {
        state.help_visible = false;
        return true;
    }
    if menu::handle_open_menu(state, params, ublx_opts, action) {
        return true;
    }
    if menu::try_open_open_menu(state, right_content, action) {
        return true;
    }
    if lens::try_open_lens_menu(state, right_content, action) {
        return true;
    }
    if menu::try_open_space_menu(state, view, right_content, action) {
        return true;
    }
    false
}

pub fn handle_ublx_input(
    state: &mut UblxState,
    view: &ViewData,
    right_content: &RightPaneContent,
    theme_ctx: applets::theme_selector::ThemeContext<'_>,
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

    if dispatch_modal_handlers(
        state,
        view,
        right_content,
        theme_ctx,
        params,
        ublx_opts,
        ModalInput { e, action },
    ) {
        return Ok(false);
    }

    if matches!(action, UblxAction::ThemeSelector) {
        applets::theme_selector::open(state, theme_ctx);
        return Ok(false);
    }
    if matches!(action, UblxAction::ReloadConfig) {
        applets::settings::apply_config_reload(
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
    let ctx = state_transitions::UblxActionContext::new(view, right_content);
    Ok(ctx.apply_action_to_state(state, action, tabs.has_duplicates, tabs.has_lenses))
}
