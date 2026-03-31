use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use std::io;

use crate::app::RunUblxParams;
use crate::config::{LayoutOverlay, UblxOpts};
use crate::handlers::{applets, leave_terminal_for_editor, state_transitions};
use crate::layout::setup::{MainMode, RightPaneContent, StartupPromptPhase, UblxState, ViewData};
use crate::ui::{
    MainTabFlags,
    consts::{MAIN_TAB_KEYS, UI_CONSTANTS, UI_STRINGS},
    file_ops, keymap, lens, menu, mouse,
};

#[derive(Clone)]
pub struct InputContext<'a> {
    pub view: &'a ViewData,
    pub right_content: &'a RightPaneContent,
    pub theme_ctx: applets::theme_selector::ThemeContext,
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
    state_mut: &mut UblxState,
    view_ref: &ViewData,
    right_content_ref: &RightPaneContent,
    theme_ctx_ref: &applets::theme_selector::ThemeContext,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    input: ModalInput,
) -> bool {
    let ModalInput { e, action } = input;
    // Table-style: each line is (guard / handler returns true) → we handled the event.
    if applets::first_run::handle_startup_prompt(state_mut, params_mut, ublx_opts_mut, action) {
        return true;
    }
    if file_ops::handle_file_delete_confirm(state_mut, params_mut, action) {
        return true;
    }
    if file_ops::handle_file_rename_input(state_mut, params_mut, e) {
        return true;
    }
    if lens::handle_lens_name_input(state_mut, params_mut, e) {
        return true;
    }
    if lens::handle_lens_rename_input(state_mut, params_mut, e) {
        return true;
    }
    if lens::handle_lens_delete_confirm(state_mut, params_mut, action) {
        return true;
    }
    if applets::enhance_policy::handle_enhance_policy_menu(
        state_mut,
        params_mut,
        ublx_opts_mut,
        action,
    ) {
        return true;
    }
    if menu::handle_space_menu(state_mut, view_ref, params_mut, ublx_opts_mut, action) {
        return true;
    }
    if lens::handle_lens_menu(state_mut, params_mut, action) {
        return true;
    }
    if state_mut.theme.selector_visible {
        applets::theme_selector::handle_key(
            state_mut,
            params_mut,
            ublx_opts_mut,
            theme_ctx_ref,
            action,
        );
        return true;
    }
    if state_mut.chrome.help_visible {
        state_mut.chrome.help_visible = false;
        return true;
    }
    if menu::handle_open_menu(state_mut, params_mut, ublx_opts_mut, action) {
        return true;
    }
    if menu::try_enhance_with_zahir(
        state_mut,
        right_content_ref,
        params_mut,
        ublx_opts_mut,
        action,
    ) {
        return true;
    }
    if menu::try_open_open_menu(state_mut, right_content_ref, action) {
        return true;
    }
    if lens::try_open_lens_menu(state_mut, right_content_ref, action) {
        return true;
    }
    if menu::try_open_space_menu(state_mut, view_ref, right_content_ref, action) {
        return true;
    }
    false
}

fn try_open_settings_editor_from_menu(
    state_mut: &mut UblxState,
    action: keymap::UblxAction,
    ublx_opts_ref: &UblxOpts,
) -> bool {
    if state_mut.main_mode != MainMode::Settings || !matches!(action, keymap::UblxAction::OpenMenu)
    {
        return false;
    }
    if let Some(ref path) = state_mut.settings.editing_path
        && let Some(ed) = applets::opener::editor_for_open(ublx_opts_ref.editor_path.as_deref())
    {
        let _ = leave_terminal_for_editor();
        let _ = applets::opener::open_in_editor(&ed, path);
        state_mut.session.tick.refresh_terminal_after_editor = true;
    }
    true
}

/// After modals: Settings digit buffers, theme picker, Settings keys, reload, search clear.
fn handle_post_modal_chrome_keys(
    state_mut: &mut UblxState,
    theme_ctx_ref: &applets::theme_selector::ThemeContext,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    e: KeyEvent,
    action: keymap::UblxAction,
) -> bool {
    if state_mut.main_mode == MainMode::Settings
        && applets::settings::handle_layout_text_key(state_mut, e)
    {
        return true;
    }
    if matches!(action, keymap::UblxAction::ThemeSelector) && !state_mut.theme.selector_visible {
        applets::theme_selector::open(state_mut, theme_ctx_ref);
        return true;
    }
    if state_mut.main_mode == MainMode::Settings
        && applets::settings::handle_key(state_mut, params_mut, ublx_opts_mut, action)
    {
        return true;
    }
    if matches!(action, keymap::UblxAction::ReloadConfig) {
        applets::settings::apply_config_reload(
            params_mut,
            ublx_opts_mut,
            state_mut,
            Some(UI_STRINGS.toasts.config_reloaded),
        );
        return true;
    }
    if matches!(action, keymap::UblxAction::ViewerFindClear) {
        applets::viewer_find::clear(state_mut);
        return true;
    }
    if matches!(action, keymap::UblxAction::SearchClear) {
        state_mut.search.query.clear();
        state_mut.search.active = false;
        return true;
    }
    false
}

fn handle_viewer_find_typing(state_mut: &mut UblxState, action: keymap::UblxAction) -> bool {
    if !state_mut.viewer_find.active {
        return false;
    }
    match action {
        keymap::UblxAction::ViewerFindSubmit => {
            state_mut.viewer_find.active = false;
            state_mut.viewer_find.committed = true;
        }
        keymap::UblxAction::ViewerFindBackspace => {
            state_mut.viewer_find.query.pop();
            state_mut.viewer_find.last_sync_token = None;
        }
        keymap::UblxAction::ViewerFindChar(c) => {
            state_mut.viewer_find.query.push(c);
            state_mut.viewer_find.last_sync_token = None;
        }
        _ => return false,
    }
    keymap::viewer_find_consumes(action)
}

fn handle_active_search_key(state_mut: &mut UblxState, action: keymap::UblxAction) -> bool {
    if !state_mut.search.active {
        return false;
    }
    match action {
        keymap::UblxAction::SearchSubmit => state_mut.search.active = false,
        keymap::UblxAction::SearchBackspace => {
            state_mut.search.query.pop();
        }
        keymap::UblxAction::SearchChar(c) => state_mut.search.query.push(c),
        _ => {}
    }
    keymap::search_consumes(action)
}

fn key_action_context_for(state: &UblxState, tabs: MainTabFlags) -> keymap::KeyActionContext {
    let has_search_filter = !state.search.query.is_empty();
    keymap::KeyActionContext {
        search: keymap::KeySearchState {
            active: state.search.active,
            has_filter: has_search_filter,
        },
        viewer_find_typing: state.viewer_find.active,
        viewer_find_committed: state.viewer_find.committed,
        allow_viewer_find: !matches!(state.main_mode, MainMode::Settings),
        last_key_for_double: state.last_key_for_double,
        tabs: keymap::KeyOptionalTabs {
            duplicates: tabs.has_duplicates,
            lenses: tabs.has_lenses,
        },
        tab_keys: MAIN_TAB_KEYS,
    }
}

/// Yes/no overlays and space-menu letter hotkeys override the mapped action.
fn apply_overlay_key_overrides(
    state_mut: &mut UblxState,
    e: KeyEvent,
    action: &mut keymap::UblxAction,
) {
    let binary_y_n_overlay = state_mut.file_delete_confirm.visible
        || state_mut.lens_confirm.delete_visible
        || state_mut.enhance_policy_menu.visible
        || matches!(
            state_mut.startup_prompt.as_ref().map(|s| &s.phase),
            Some(StartupPromptPhase::PreviousSettings { .. } | StartupPromptPhase::Enhance { .. })
        );
    if binary_y_n_overlay
        && !e.modifiers.contains(KeyModifiers::CONTROL)
        && !e.modifiers.contains(KeyModifiers::SHIFT)
        && let KeyCode::Char(c) = e.code
    {
        match c.to_ascii_lowercase() {
            'y' => {
                *action = keymap::UblxAction::ConfirmYes;
                state_mut.last_key_for_double = None;
            }
            'n' => {
                *action = keymap::UblxAction::ConfirmNo;
                state_mut.last_key_for_double = None;
            }
            _ => {}
        }
    }

    if state_mut.space_menu.visible
        && let Some(ref kind) = state_mut.space_menu.kind
        && !e.modifiers.contains(KeyModifiers::CONTROL)
        && !e.modifiers.contains(KeyModifiers::SHIFT)
        && let KeyCode::Char(c) = e.code
        && let Some(idx) = menu::space_menu_hotkey_to_index(kind, c)
    {
        *action = keymap::UblxAction::SpaceMenuHotkeySelect(idx);
        state_mut.last_key_for_double = None;
    }
}

fn apply_action_with_settings_enter(
    state_mut: &mut UblxState,
    view_ref: &ViewData,
    right_content_ref: &RightPaneContent,
    action: keymap::UblxAction,
    tabs: MainTabFlags,
    params_mut: &mut RunUblxParams<'_>,
) -> bool {
    let mode_before = state_mut.main_mode;
    let ctx = state_transitions::UblxActionContext::new(view_ref, right_content_ref);
    let quit = ctx.apply_action_to_state(state_mut, action, tabs.has_duplicates, tabs.has_lenses);
    if state_mut.main_mode == MainMode::Settings && mode_before != MainMode::Settings {
        applets::settings::on_enter_settings(state_mut, params_mut);
    }
    quit
}

/// Poll for one key event, map it to a [`UblxAction`], run modal handlers, then apply the action to state.
/// Returns `Ok(true)` when the event was handled and the main loop should skip further processing for this tick;
/// `Ok(false)` when there was no key, a non-key event, or the event was consumed without requesting quit semantics.
///
/// # Errors
///
/// Returns [`io::Error`] when `crossterm` fails to poll or read the next event (`event::poll` / `event::read`).
pub fn handle_ublx_input(
    state_mut: &mut UblxState,
    ctx: InputContext<'_>,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
) -> io::Result<bool> {
    let InputContext {
        view: view_ref,
        right_content: right_content_ref,
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
            state_mut,
            me,
            mouse::MouseContext {
                view: view_ref,
                right_content: right_content_ref,
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
    let result = keymap::key_action_setup(e, &key_action_context_for(state_mut, tabs));
    state_mut.last_key_for_double = result.last_key_for_double;
    let mut action = result.action;

    apply_overlay_key_overrides(state_mut, e, &mut action);

    if try_open_settings_editor_from_menu(state_mut, action, &*ublx_opts_mut) {
        return Ok(false);
    }

    if dispatch_modal_handlers(
        state_mut,
        view_ref,
        right_content_ref,
        &theme_ctx,
        params_mut,
        ublx_opts_mut,
        ModalInput { e, action },
    ) {
        return Ok(false);
    }

    if handle_post_modal_chrome_keys(state_mut, &theme_ctx, params_mut, ublx_opts_mut, e, action) {
        return Ok(false);
    }

    if handle_viewer_find_typing(state_mut, action) {
        return Ok(false);
    }

    if handle_active_search_key(state_mut, action) {
        return Ok(false);
    }

    Ok(apply_action_with_settings_enter(
        state_mut,
        view_ref,
        right_content_ref,
        action,
        tabs,
        params_mut,
    ))
}
