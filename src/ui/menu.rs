//! Open menu (Shift+O) and spacebar context menu input handling.

use crate::config::UblxOpts;
use crate::handlers::{applets, core};
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::{
    MainMode, PanelFocus, RightPaneContent, SpaceMenuKind, UblxState, ViewData,
};
use crate::ui::{keymap::UblxAction, show_operation_toast};

/// Handle action when Open menu (Terminal/GUI) is visible. Returns true if handled.
pub fn handle_open_menu(
    state: &mut UblxState,
    params: &RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    action: UblxAction,
) -> bool {
    if !state.open_menu.visible {
        return false;
    }
    match action {
        UblxAction::Quit | UblxAction::SearchClear => state.close_open_menu(),
        UblxAction::MoveDown => {
            let max_idx = usize::from(state.open_menu.can_terminal);
            state.open_menu.selected_index = (state.open_menu.selected_index + 1).min(max_idx);
        }
        UblxAction::MoveUp => {
            state.open_menu.selected_index = state.open_menu.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            if let Some(ref rel_path) = state.open_menu.path {
                let full_path = params.dir_to_ublx.join(rel_path);
                let use_terminal =
                    state.open_menu.can_terminal && state.open_menu.selected_index == 0;
                if use_terminal {
                    if let Some(ed) =
                        applets::opener::editor_for_open(ublx_opts.editor_path.as_deref())
                    {
                        let _ = core::leave_terminal_for_editor();
                        let _ = applets::opener::open_in_editor(&ed, &full_path);
                        state.session.refresh_terminal_after_editor = true;
                    }
                } else {
                    let _ = applets::opener::open_in_gui(&full_path);
                }
            }
            state.close_open_menu();
        }
        _ => {}
    }
    true
}

/// If action is `OpenMenu` and a file is selected, open the open menu. Returns true if opened.
/// Openable files (e.g. text) get Terminal + GUI; others (e.g. .mp3) get only Open (GUI).
pub fn try_open_open_menu(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    action: UblxAction,
) -> bool {
    if !matches!(action, UblxAction::OpenMenu) {
        return false;
    }
    if let Some(path) = right_content.viewer_path.clone() {
        state.open_open_menu(path, right_content.viewer_can_open);
        return true;
    }
    false
}

/// Handle action when spacebar context menu is visible. Returns true if handled.
pub fn handle_space_menu(
    state: &mut UblxState,
    view: &ViewData,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    action: UblxAction,
) -> bool {
    if !state.space_menu.visible {
        return false;
    }
    let item_count: usize = match &state.space_menu.kind {
        Some(SpaceMenuKind::FileActions {
            show_enhance_directory_policy,
            show_enhance_zahir,
            ..
        }) => {
            2 + usize::from(*show_enhance_directory_policy) + usize::from(*show_enhance_zahir) + 1
        }
        Some(SpaceMenuKind::LensPanelActions { .. }) => 2,
        None => 0,
    };
    match action {
        UblxAction::Quit | UblxAction::SearchClear => state.close_space_menu(),
        UblxAction::MoveDown => {
            state.space_menu.selected_index =
                (state.space_menu.selected_index + 1).min(item_count.saturating_sub(1));
        }
        UblxAction::MoveUp => {
            state.space_menu.selected_index = state.space_menu.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            let kind = state.space_menu.kind.clone();
            let idx = state.space_menu.selected_index;
            state.close_space_menu();
            if let Some(k) = kind {
                match k {
                    SpaceMenuKind::FileActions {
                        path,
                        can_open_in_terminal,
                        show_enhance_directory_policy,
                        show_enhance_zahir,
                    } => {
                        let mut i = 0usize;
                        if idx == i {
                            state.open_open_menu(path, can_open_in_terminal);
                            return true;
                        }
                        i += 1;
                        if idx == i {
                            let full = params.dir_to_ublx.join(&path);
                            if let Err(e) = applets::opener::reveal_in_file_manager(&full) {
                                log::warn!("Show in folder: {e}");
                            }
                            return true;
                        }
                        i += 1;
                        if show_enhance_directory_policy {
                            if idx == i {
                                state.enhance_policy_menu.visible = true;
                                state.enhance_policy_menu.path = Some(path);
                                state.enhance_policy_menu.selected_index = 0;
                                return true;
                            }
                            i += 1;
                        }
                        if show_enhance_zahir {
                            if idx == i {
                                if !ublx_opts.enable_enhance_all {
                                    match applets::enhance::enhance_single_path(
                                        params.dir_to_ublx,
                                        params.db_path,
                                        &path,
                                        ublx_opts,
                                    ) {
                                        Ok(()) => {
                                            state.session.reload_snapshot_rows = true;
                                            show_operation_toast(
                                                state,
                                                params,
                                                "Enhanced with ZahirScan",
                                                "enhance",
                                                log::Level::Info,
                                            );
                                        }
                                        Err(e) => {
                                            log::warn!("Enhance with ZahirScan: {e}");
                                            show_operation_toast(
                                                state,
                                                params,
                                                format!("Enhance failed: {e}"),
                                                "enhance",
                                                log::Level::Info,
                                            );
                                        }
                                    }
                                }
                                return true;
                            }
                            i += 1;
                        }
                        if idx == i {
                            if state.main_mode == MainMode::Snapshot {
                                state.open_lens_menu(path);
                            } else if let Some(lens_name) = view
                                .filtered_categories
                                .get(state.panels.category_state.selected().unwrap_or(0))
                                && crate::handlers::applets::lens::remove_path_from_lens(
                                    params.db_path,
                                    lens_name,
                                    &path,
                                )
                                .is_ok()
                            {
                                show_operation_toast(
                                    state,
                                    params,
                                    format!("Removed from lens \"{lens_name}\""),
                                    "lens",
                                    log::Level::Info,
                                );
                            }
                        }
                    }
                    SpaceMenuKind::LensPanelActions { lens_name } => {
                        if idx == 0 {
                            state.lens_confirm.rename_input = Some((lens_name.clone(), lens_name));
                        } else {
                            state.open_lens_delete_confirm(lens_name);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    true
}

/// If action is `SpaceMenu` and context allows, open the space menu. Returns true if opened.
pub fn try_open_space_menu(
    state: &mut UblxState,
    view: &ViewData,
    right_content: &RightPaneContent,
    action: UblxAction,
) -> bool {
    if !matches!(action, UblxAction::SpaceMenu) {
        return false;
    }
    if !matches!(state.main_mode, MainMode::Snapshot | MainMode::Lenses)
        || state.space_menu.visible
        || state.enhance_policy_menu.visible
        || state.lens_confirm.rename_input.is_some()
        || state.lens_confirm.delete_visible
        || state.open_menu.visible
        || state.lens_menu.visible
        || state.lens_menu.name_input.is_some()
    {
        return false;
    }
    if state.panels.focus == PanelFocus::Contents {
        if let Some(ref path) = right_content.viewer_path {
            state.open_space_menu(SpaceMenuKind::FileActions {
                path: path.clone(),
                can_open_in_terminal: right_content.viewer_can_open,
                show_enhance_directory_policy: right_content.viewer_offer_enhance_directory_policy,
                show_enhance_zahir: right_content.viewer_offer_enhance_zahir,
            });
            return true;
        }
    } else if state.main_mode == MainMode::Lenses
        && !view.filtered_categories.is_empty()
        && let Some(lens_name) = view
            .filtered_categories
            .get(state.panels.category_state.selected().unwrap_or(0))
            .cloned()
    {
        state.open_space_menu(SpaceMenuKind::LensPanelActions { lens_name });
        return true;
    }
    false
}
