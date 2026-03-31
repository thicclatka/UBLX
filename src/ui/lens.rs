//! Lens-related input handling: lens name input, rename, delete confirm, and lens menu.

use crossterm::event::KeyCode;

use crate::app::RunUblxParams;
use crate::handlers::applets::lens as lens_applet;
use crate::layout::setup::{RightPaneContent, UblxState};
use crate::ui::{UI_STRINGS, file_ops::modal_open, keymap::UblxAction, show_operation_toast};

/// Handle key when user is typing a new lens name (Create New Lens). Returns true if key was consumed.
pub fn handle_lens_name_input(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    e: crossterm::event::KeyEvent,
) -> bool {
    let Some(ref mut name) = state.lens_menu.name_input else {
        return false;
    };
    match e.code {
        KeyCode::Char(c) => name.push(c),
        KeyCode::Backspace => {
            name.pop();
        }
        KeyCode::Enter => {
            let path = state.lens_menu.path.clone().unwrap_or_default();
            let name_trimmed = name.trim().to_string();
            state.lens_menu.name_input = None;
            state.close_lens_menu();
            if !name_trimmed.is_empty() {
                let created = lens_applet::create_lens(params.db_path, &name_trimmed).is_ok();
                if lens_applet::add_path_to_lens(params.db_path, &name_trimmed, &path).is_ok() {
                    if created && !params.lens_names.contains(&name_trimmed) {
                        params.lens_names.push(name_trimmed.clone());
                    }
                    let msg = if created {
                        UI_STRINGS
                            .lens
                            .toast_created_and_added_file
                            .replace("{LENS}", &name_trimmed)
                    } else {
                        UI_STRINGS
                            .lens
                            .toast_added_to_lens
                            .replace("{LENS}", &name_trimmed)
                    };
                    show_operation_toast(state, params, msg, "lens", log::Level::Info);
                }
            }
        }
        KeyCode::Esc => state.lens_menu.name_input = None,
        _ => {}
    }
    true
}

/// Handle key when user is renaming a lens. Returns true if key was consumed.
pub fn handle_lens_rename_input(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    e: crossterm::event::KeyEvent,
) -> bool {
    let Some((target_name, mut current)) = state.lens_confirm.rename_input.take() else {
        return false;
    };
    match e.code {
        KeyCode::Char(c) => {
            current.push(c);
            state.lens_confirm.rename_input = Some((target_name, current));
        }
        KeyCode::Backspace => {
            current.pop();
            state.lens_confirm.rename_input = Some((target_name, current));
        }
        KeyCode::Enter => {
            let new_name = current.trim().to_string();
            if !new_name.is_empty()
                && new_name != target_name
                && lens_applet::rename_lens(params.db_path, &target_name, &new_name).is_ok()
            {
                if let Some(i) = params.lens_names.iter().position(|n| n == &target_name) {
                    params.lens_names[i].clone_from(&new_name);
                }
                show_operation_toast(
                    state,
                    params,
                    UI_STRINGS
                        .lens
                        .toast_renamed_to
                        .replace("{LENS}", &new_name),
                    "lens",
                    log::Level::Info,
                );
            }
        }
        KeyCode::Esc => {}
        _ => state.lens_confirm.rename_input = Some((target_name, current)),
    }
    true
}

/// Handle action when delete-lens confirmation is visible. Returns true if handled.
pub fn handle_lens_delete_confirm(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    action: UblxAction,
) -> bool {
    if !state.lens_confirm.delete_visible {
        return false;
    }
    match action {
        UblxAction::ConfirmYes => {
            state.lens_confirm.delete_selected = 0;
            return handle_lens_delete_confirm(state, params, UblxAction::SearchSubmit);
        }
        UblxAction::ConfirmNo => {
            state.lens_confirm.delete_selected = 1;
            return handle_lens_delete_confirm(state, params, UblxAction::SearchSubmit);
        }
        UblxAction::Quit | UblxAction::SearchClear => state.close_lens_delete_confirm(),
        UblxAction::MoveDown => {
            state.lens_confirm.delete_selected = 1.min(state.lens_confirm.delete_selected + 1);
        }
        UblxAction::MoveUp => {
            state.lens_confirm.delete_selected =
                state.lens_confirm.delete_selected.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            let lens_name = state.lens_confirm.delete_lens_name.take();
            let selected = state.lens_confirm.delete_selected;
            state.close_lens_delete_confirm();
            if selected == 0
                && let Some(name) = lens_name
                && lens_applet::delete_lens(params.db_path, &name).is_ok()
            {
                params.lens_names.retain(|n| n != &name);
                show_operation_toast(
                    state,
                    params,
                    UI_STRINGS.lens.toast_deleted_lens.replace("{LENS}", &name),
                    "lens",
                    log::Level::Info,
                );
            }
        }
        _ => {}
    }
    true
}

/// Handle action when lens menu (Add to lens) is visible. Returns true if handled.
pub fn handle_lens_menu(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    action: UblxAction,
) -> bool {
    if !state.lens_menu.visible {
        return false;
    }
    match action {
        UblxAction::Quit | UblxAction::SearchClear => state.close_lens_menu(),
        UblxAction::MoveDown => {
            let max_idx = params.lens_names.len();
            state.lens_menu.selected_index = (state.lens_menu.selected_index + 1).min(max_idx);
        }
        UblxAction::MoveUp => {
            state.lens_menu.selected_index = state.lens_menu.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            if let Some(ref path) = state.lens_menu.path {
                if state.lens_menu.selected_index == 0 {
                    state.lens_menu.name_input = Some(String::new());
                } else if let Some(lens_name) =
                    params.lens_names.get(state.lens_menu.selected_index - 1)
                {
                    if lens_applet::add_path_to_lens(params.db_path, lens_name, path).is_ok() {
                        show_operation_toast(
                            state,
                            params,
                            UI_STRINGS
                                .lens
                                .toast_added_to_lens
                                .replace("{LENS}", lens_name),
                            "lens",
                            log::Level::Info,
                        );
                    }
                    state.close_lens_menu();
                }
            }
        }
        _ => {}
    }
    true
}

/// If action is `LensMenu` and a file is selected, open the lens menu. Returns true if opened.
pub fn try_open_lens_menu(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    action: UblxAction,
) -> bool {
    if !matches!(action, UblxAction::LensMenu) {
        return false;
    }
    if modal_open(state) {
        return false;
    }
    if let Some(path) = right_content.snap_meta.path.clone() {
        state.open_lens_menu(path);
        return true;
    }
    false
}
