//! File rename / delete from the space menu (submenu + prompts).

use crossterm::event::KeyCode;

use crate::app::RunUblxParams;
use crate::handlers::applets::file_ops as file_ops_applet;
use crate::layout::setup::UblxState;
use crate::ui::{UI_STRINGS, keymap::UblxAction, show_operation_toast};

/// True while a file rename/delete modal is active (blocks opening other menus).
#[must_use]
pub fn modal_open(state: &UblxState) -> bool {
    state.file_rename_input.is_some() || state.file_delete_confirm.visible
}

/// Delete confirmation: Yes / No (index 0 = delete).
pub fn handle_file_delete_confirm(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    action: UblxAction,
) -> bool {
    if !state.file_delete_confirm.visible {
        return false;
    }
    match action {
        UblxAction::ConfirmYes => {
            state.file_delete_confirm.selected_index = 0;
            return handle_file_delete_confirm(state, params, UblxAction::SearchSubmit);
        }
        UblxAction::ConfirmNo => {
            state.file_delete_confirm.selected_index = 1;
            return handle_file_delete_confirm(state, params, UblxAction::SearchSubmit);
        }
        UblxAction::Quit | UblxAction::SearchClear => state.close_file_delete_confirm(),
        UblxAction::MoveDown => {
            state.file_delete_confirm.selected_index =
                1.min(state.file_delete_confirm.selected_index + 1);
        }
        UblxAction::MoveUp => {
            state.file_delete_confirm.selected_index =
                state.file_delete_confirm.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            let rel = state.file_delete_confirm.rel_path.take();
            let selected = state.file_delete_confirm.selected_index;
            state.close_file_delete_confirm();
            if selected == 0
                && let Some(p) = rel
            {
                match file_ops_applet::delete_entry_under_root(
                    params.dir_to_ublx,
                    params.db_path,
                    &p,
                ) {
                    Ok(()) => {
                        state.session.reload.snapshot_rows = true;
                        state.cached_tree = None;
                        state.viewer_disk_cache = None;
                        show_operation_toast(
                            state,
                            params,
                            UI_STRINGS.toasts.file_deleted,
                            "file",
                            log::Level::Info,
                        );
                    }
                    Err(e) => {
                        log::warn!("Delete entry: {e}");
                        show_operation_toast(
                            state,
                            params,
                            format!("{}{e}", UI_STRINGS.toasts.file_ops_failed_prefix),
                            "file",
                            log::Level::Warn,
                        );
                    }
                }
            }
        }
        _ => {}
    }
    true
}

/// Basename rename (centered text-input popup).
pub fn handle_file_rename_input(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    e: crossterm::event::KeyEvent,
) -> bool {
    let Some((rel_path, mut current)) = state.file_rename_input.take() else {
        return false;
    };
    match e.code {
        KeyCode::Char(c) => {
            current.push(c);
            state.file_rename_input = Some((rel_path, current));
        }
        KeyCode::Backspace => {
            current.pop();
            state.file_rename_input = Some((rel_path, current));
        }
        KeyCode::Enter => {
            match file_ops_applet::rename_entry_under_root(
                params.dir_to_ublx,
                params.db_path,
                &rel_path,
                &current,
            ) {
                Ok(new_rel) => {
                    state.session.reload.snapshot_rows = true;
                    state.cached_tree = None;
                    state.viewer_disk_cache = None;
                    show_operation_toast(
                        state,
                        params,
                        UI_STRINGS.toasts.file_renamed.replace("{PATH}", &new_rel),
                        "file",
                        log::Level::Info,
                    );
                }
                Err(e) => {
                    log::warn!("Rename entry: {e}");
                    show_operation_toast(
                        state,
                        params,
                        format!("{}{e}", UI_STRINGS.toasts.file_ops_failed_prefix),
                        "file",
                        log::Level::Warn,
                    );
                }
            }
        }
        KeyCode::Esc => {}
        _ => state.file_rename_input = Some((rel_path, current)),
    }
    true
}
