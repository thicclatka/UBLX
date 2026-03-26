use crossterm::event::{self, Event, KeyEvent};
use ratatui::layout::Rect;
use std::io;

use crate::app::RunUblxParams;
use crate::config::{LayoutOverlay, UblxOpts};
use crate::handlers::{applets, state_transitions};
use crate::layout::setup::{RightPaneContent, UblxState, ViewData};
use crate::ui::{
    MainTabFlags,
    consts::{UI_CONSTANTS, UI_STRINGS},
    keymap, lens, menu, mouse,
};

#[derive(Clone, Copy)]
pub struct InputContext<'a> {
    pub view: &'a ViewData,
    pub right_content: &'a RightPaneContent,
    pub theme_ctx: applets::theme_selector::ThemeContext<'a>,
    pub frame_area: Rect,
    pub layout: &'a LayoutOverlay,
    pub tabs: MainTabFlags,
}

/// Key event and resolved action passed into modal handlers (keeps [`dispatch_modal_handlers`] under clippy’s arg limit).
#[derive(Clone, Copy)]
struct ModalInput {
    e: KeyEvent,
    action: keymap::UblxAction,
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
    if applets::first_run::handle_initial_prompt(state, params, ublx_opts, action) {
        return true;
    }
    if lens::handle_lens_name_input(state, params, e) {
        return true;
    }
    if lens::handle_lens_rename_input(state, params, e) {
        return true;
    }
    if lens::handle_lens_delete_confirm(state, params, action) {
        return true;
    }
    if applets::enhance_policy::handle_enhance_policy_menu(state, params, ublx_opts, action) {
        return true;
    }
    if menu::handle_space_menu(state, view, params, ublx_opts, action) {
        return true;
    }
    if lens::handle_lens_menu(state, params, action) {
        return true;
    }
    if state.theme.selector_visible {
        applets::theme_selector::handle_key(state, params, theme_ctx, action);
        return true;
    }
    if state.chrome.help_visible {
        state.chrome.help_visible = false;
        return true;
    }
    if menu::handle_open_menu(state, params, ublx_opts, action) {
        return true;
    }
    if menu::try_enhance_with_zahir(state, right_content, params, ublx_opts, action) {
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

/// Poll for one key event, map it to a [`UblxAction`], run modal handlers, then apply the action to state.
/// Returns `Ok(true)` when the event was handled and the main loop should skip further processing for this tick;
/// `Ok(false)` when there was no key, a non-key event, or the event was consumed without requesting quit semantics.
///
/// # Errors
///
/// Returns [`io::Error`] when `crossterm` fails to poll or read the next event (`event::poll` / `event::read`).
pub fn handle_ublx_input(
    state: &mut UblxState,
    ctx: InputContext<'_>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) -> io::Result<bool> {
    let InputContext {
        view,
        right_content,
        theme_ctx,
        frame_area,
        layout,
        tabs,
    } = ctx;
    if !event::poll(std::time::Duration::from_millis(UI_CONSTANTS.input_poll_ms))? {
        return Ok(false);
    }
    let ev = event::read()?;
    if let Event::Mouse(me) = ev {
        let _handled = mouse::handle_mouse_event(
            state,
            me,
            mouse::MouseContext {
                view,
                right_content,
                frame_area,
                layout,
                tabs,
            },
        );
        return Ok(false);
    }
    let Event::Key(e) = ev else {
        return Ok(false);
    };
    let has_search_filter = !state.search.query.is_empty();
    let result = keymap::key_action_setup(
        e,
        &keymap::KeyActionContext {
            search: keymap::KeySearchState {
                active: state.search.active,
                has_filter: has_search_filter,
            },
            last_key_for_double: state.last_key_for_double,
            tabs: keymap::KeyOptionalTabs {
                duplicates: tabs.has_duplicates,
                lenses: tabs.has_lenses,
            },
        },
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

    if matches!(action, keymap::UblxAction::ThemeSelector) {
        applets::theme_selector::open(state, theme_ctx);
        return Ok(false);
    }
    if matches!(action, keymap::UblxAction::ReloadConfig) {
        applets::settings::apply_config_reload(
            params,
            ublx_opts,
            state,
            Some(UI_STRINGS.toasts.config_reloaded),
        );
        return Ok(false);
    }
    if matches!(action, keymap::UblxAction::SearchClear) {
        state.search.query.clear();
        state.search.active = false;
        return Ok(false);
    }
    if state.search.active {
        match action {
            keymap::UblxAction::SearchSubmit => state.search.active = false,
            keymap::UblxAction::SearchBackspace => {
                state.search.query.pop();
            }
            keymap::UblxAction::SearchChar(c) => state.search.query.push(c),
            _ => {}
        }
        if keymap::search_consumes(action) {
            return Ok(false);
        }
    }
    let ctx = state_transitions::UblxActionContext::new(view, right_content);
    Ok(ctx.apply_action_to_state(state, action, tabs.has_duplicates, tabs.has_lenses))
}
