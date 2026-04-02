//! File rename / delete from the quick actions menu (spacebar) (submenu + prompts).

use crossterm::event::KeyCode;

use crate::app::RunUblxParams;
use crate::config::UblxOpts;
use crate::handlers::leave_terminal_for_editor;
use crate::layout::setup::{MainMode, UblxState};
use crate::modules::{file_ops as file_ops_applet, opener};
use crate::ui::{UI_STRINGS, keymap::UblxAction, show_operation_toast};

/// True while a file rename/delete modal is active (blocks opening other menus).
#[must_use]
pub fn modal_open(state: &UblxState) -> bool {
    state.file_rename_input.is_some() || state.file_delete_confirm.visible
}

fn invalidate_file_list_caches(state: &mut UblxState) {
    state.session.reload.snapshot_rows = true;
    state.cached_tree = None;
    state.viewer_disk_cache = None;
}

fn run_bulk_delete(state: &mut UblxState, params: &mut RunUblxParams<'_>, paths: Vec<String>) {
    let mut last_err: Option<String> = None;
    let mut ok_count = 0usize;
    for p in paths {
        match file_ops_applet::delete_entry_under_root(&params.dir_to_ublx, &params.db_path, &p) {
            Ok(()) => ok_count += 1,
            Err(e) => {
                log::warn!("Delete entry {p}: {e}");
                last_err = Some(e.to_string());
                break;
            }
        }
    }
    invalidate_file_list_caches(state);
    state.multiselect.clear();
    if let Some(err) = last_err {
        show_operation_toast(
            state,
            params,
            format!("Deleted {ok_count}; then failed: {err}"),
            "file",
            log::Level::Warn,
        );
    } else {
        show_operation_toast(
            state,
            params,
            format!("Deleted {ok_count}"),
            "file",
            log::Level::Info,
        );
    }
}

fn run_single_delete(state: &mut UblxState, params: &mut RunUblxParams<'_>, p: &str) {
    match file_ops_applet::delete_entry_under_root(&params.dir_to_ublx, &params.db_path, p) {
        Ok(()) => {
            invalidate_file_list_caches(state);
            if state.main_mode == MainMode::Duplicates {
                state.session.reload.duplicate_groups = true;
                state.duplicate_ignored_paths.remove(p);
            }
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
            let bulk = state.file_delete_confirm.bulk_paths.take();
            let rel = state.file_delete_confirm.rel_path.take();
            let selected = state.file_delete_confirm.selected_index;
            state.close_file_delete_confirm();
            if selected != 0 {
                return true;
            }
            if let Some(paths) = bulk {
                run_bulk_delete(state, params, paths);
            } else if let Some(p) = rel {
                run_single_delete(state, params, &p);
            }
        }
        _ => {}
    }
    true
}

fn apply_bulk_rename_pairs(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    pairs: Vec<(String, String)>,
) {
    let mut ok = 0usize;
    let mut last_err: Option<String> = None;
    for (rel, base) in pairs {
        match file_ops_applet::rename_entry_under_root(
            &params.dir_to_ublx,
            &params.db_path,
            &rel,
            &base,
        ) {
            Ok(_) => {
                ok += 1;
                invalidate_file_list_caches(state);
            }
            Err(e) => {
                log::warn!("Bulk rename {rel}: {e}");
                last_err = Some(e.to_string());
                break;
            }
        }
    }
    state.multiselect.clear();
    if let Some(err) = last_err {
        show_operation_toast(
            state,
            params,
            format!("Renamed {ok}; then failed: {err}"),
            "file",
            log::Level::Warn,
        );
    } else {
        show_operation_toast(
            state,
            params,
            UI_STRINGS
                .toasts
                .bulk_renamed_n
                .replace("{N}", &ok.to_string()),
            "file",
            log::Level::Info,
        );
    }
}

/// Multi-select bulk rename: write paths to a temp file, open `$EDITOR` / `editor_path`, apply line-by-line.
pub fn bulk_rename_via_editor(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    paths: &[String],
    ublx_opts: &UblxOpts,
) {
    if paths.is_empty() {
        return;
    }
    let Some(ed) = opener::editor_for_open(ublx_opts.editor_path.as_deref()) else {
        show_operation_toast(
            state,
            params,
            UI_STRINGS.toasts.bulk_rename_no_editor,
            "file",
            log::Level::Warn,
        );
        return;
    };

    let temp_path = std::env::temp_dir().join(format!(
        "ublx-bulk-rename-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));

    let content = paths.join("\n") + "\n";
    if let Err(e) = std::fs::write(&temp_path, content) {
        log::warn!("bulk rename temp file: {e}");
        show_operation_toast(
            state,
            params,
            format!("{}{e}", UI_STRINGS.toasts.file_ops_failed_prefix),
            "file",
            log::Level::Warn,
        );
        return;
    }

    let _ = leave_terminal_for_editor();
    let editor_result = opener::open_in_editor(&ed, &temp_path);
    state.session.tick.refresh_terminal_after_editor = true;

    let read_back = std::fs::read_to_string(&temp_path);
    let _ = std::fs::remove_file(&temp_path);

    if editor_result.is_err() {
        show_operation_toast(
            state,
            params,
            UI_STRINGS.toasts.bulk_rename_editor_failed,
            "file",
            log::Level::Warn,
        );
        return;
    }

    let text = match read_back {
        Ok(t) => t,
        Err(e) => {
            log::warn!("bulk rename read: {e}");
            show_operation_toast(
                state,
                params,
                format!("{}{e}", UI_STRINGS.toasts.file_ops_failed_prefix),
                "file",
                log::Level::Warn,
            );
            return;
        }
    };

    let pairs = match file_ops_applet::parse_bulk_rename_lines(paths, &text) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("bulk rename parse: {e}");
            show_operation_toast(
                state,
                params,
                format!("Bulk rename: {e}"),
                "file",
                log::Level::Warn,
            );
            return;
        }
    };

    if pairs.is_empty() {
        state.multiselect.clear();
        show_operation_toast(
            state,
            params,
            UI_STRINGS.toasts.bulk_rename_no_changes,
            "file",
            log::Level::Info,
        );
        return;
    }

    apply_bulk_rename_pairs(state, params, pairs);
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
                &params.dir_to_ublx,
                &params.db_path,
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
