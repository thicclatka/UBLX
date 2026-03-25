//! Open menu (Shift+O) and spacebar context menu input handling.

use crate::config::UblxOpts;
use crate::handlers::applets::lens;
use crate::handlers::{applets, core};
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::{
    MainMode, PanelFocus, RightPaneContent, SpaceMenuKind, UblxState, ViewData,
};
use crate::ui::{keymap::UblxAction, show_operation_toast};

fn open_menu_on_submit(
    state: &mut UblxState,
    params: &RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) {
    if let Some(rel_path) = state.open_menu.path.as_ref() {
        let full_path = params.dir_to_ublx.join(rel_path);
        let terminal_choice = state.open_menu.can_terminal && state.open_menu.selected_index == 0;
        if terminal_choice {
            if let Some(ed) = applets::opener::editor_for_open(ublx_opts.editor_path.as_deref()) {
                let _ = core::leave_terminal_for_editor();
                let _ = applets::opener::open_in_editor(&ed, &full_path);
                state.session.tick.refresh_terminal_after_editor = true;
            }
        } else {
            let _ = applets::opener::open_in_gui(&full_path);
        }
    }
    state.close_open_menu();
}

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
        UblxAction::SearchSubmit => open_menu_on_submit(state, params, ublx_opts),
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

/// [`SpaceMenuKind::FileActions`] + selected row (for submit handling).
struct SpaceMenuFileActionsOp {
    path: String,
    can_open_in_terminal: bool,
    show_enhance_directory_policy: bool,
    show_enhance_zahir: bool,
    idx: usize,
}

#[must_use]
fn space_menu_item_count(kind: Option<&SpaceMenuKind>) -> usize {
    match kind {
        Some(SpaceMenuKind::FileActions {
            show_enhance_directory_policy,
            show_enhance_zahir,
            ..
        }) => {
            2 + usize::from(*show_enhance_directory_policy) + usize::from(*show_enhance_zahir) + 1
        }
        Some(SpaceMenuKind::LensPanelActions { .. }) => 2,
        None => 0,
    }
}

/// Indices for file space-menu rows: Open, Reveal, optional policy, optional Zahir, Lens.
fn file_space_menu_indices(
    show_enhance_directory_policy: bool,
    show_enhance_zahir: bool,
) -> (usize, usize, Option<usize>, Option<usize>, usize) {
    let mut i = 0usize;
    let open = i;
    i += 1;
    let reveal = i;
    i += 1;
    let policy = show_enhance_directory_policy.then(|| {
        let j = i;
        i += 1;
        j
    });
    let zahir = show_enhance_zahir.then(|| {
        let j = i;
        i += 1;
        j
    });
    let lens = i;
    (open, reveal, policy, zahir, lens)
}

fn space_menu_enhance_zahir_if_disabled(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    path: &str,
    ublx_opts: &UblxOpts,
) {
    if ublx_opts.enable_enhance_all {
        return;
    }
    match applets::enhance::enhance_single_path(params.dir_to_ublx, params.db_path, path, ublx_opts)
    {
        Ok(()) => {
            state.session.reload.snapshot_rows = true;
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

fn space_menu_file_actions_submit(
    state: &mut UblxState,
    view: &ViewData,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    op: SpaceMenuFileActionsOp,
) {
    let SpaceMenuFileActionsOp {
        path,
        can_open_in_terminal,
        show_enhance_directory_policy,
        show_enhance_zahir,
        idx,
    } = op;

    let (open_i, reveal_i, policy_i, zahir_i, lens_i) =
        file_space_menu_indices(show_enhance_directory_policy, show_enhance_zahir);

    if idx == open_i {
        state.open_open_menu(path, can_open_in_terminal);
        return;
    }
    if idx == reveal_i {
        let full = params.dir_to_ublx.join(&path);
        if let Err(e) = applets::opener::reveal_in_file_manager(&full) {
            log::warn!("Show in folder: {e}");
        }
        return;
    }
    if policy_i == Some(idx) {
        state.enhance_policy_menu.visible = true;
        state.enhance_policy_menu.path = Some(path);
        state.enhance_policy_menu.selected_index = 0;
        return;
    }
    if zahir_i == Some(idx) {
        space_menu_enhance_zahir_if_disabled(state, params, &path, ublx_opts);
        return;
    }
    if idx != lens_i {
        return;
    }
    if state.main_mode == MainMode::Snapshot {
        state.open_lens_menu(path);
        return;
    }
    let Some(lens_name) = view
        .filtered_categories
        .get(state.panels.category_state.selected().unwrap_or(0))
    else {
        return;
    };
    if lens::remove_path_from_lens(params.db_path, lens_name, &path).is_ok() {
        show_operation_toast(
            state,
            params,
            format!("Removed from lens \"{lens_name}\""),
            "lens",
            log::Level::Info,
        );
    }
}

fn space_menu_apply_submit(
    state: &mut UblxState,
    view: &ViewData,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    kind: SpaceMenuKind,
    idx: usize,
) {
    match kind {
        SpaceMenuKind::FileActions {
            path,
            can_open_in_terminal,
            show_enhance_directory_policy,
            show_enhance_zahir,
        } => space_menu_file_actions_submit(
            state,
            view,
            params,
            ublx_opts,
            SpaceMenuFileActionsOp {
                path,
                can_open_in_terminal,
                show_enhance_directory_policy,
                show_enhance_zahir,
                idx,
            },
        ),
        SpaceMenuKind::LensPanelActions { lens_name } => match idx {
            0 => {
                state.lens_confirm.rename_input = Some((lens_name.clone(), lens_name));
            }
            _ => state.open_lens_delete_confirm(lens_name),
        },
    }
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
    let item_count = space_menu_item_count(state.space_menu.kind.as_ref());
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
                space_menu_apply_submit(state, view, params, ublx_opts, k, idx);
            }
        }
        _ => {}
    }
    true
}

#[must_use]
fn space_menu_open_blocked(state: &UblxState) -> bool {
    !matches!(state.main_mode, MainMode::Snapshot | MainMode::Lenses)
        || state.space_menu.visible
        || state.enhance_policy_menu.visible
        || state.lens_confirm.rename_input.is_some()
        || state.lens_confirm.delete_visible
        || state.open_menu.visible
        || state.lens_menu.visible
        || state.lens_menu.name_input.is_some()
}

fn try_open_file_space_menu(state: &mut UblxState, right_content: &RightPaneContent) -> bool {
    let Some(path) = right_content.viewer_path.as_ref() else {
        return false;
    };
    state.open_space_menu(SpaceMenuKind::FileActions {
        path: path.clone(),
        can_open_in_terminal: right_content.viewer_can_open,
        show_enhance_directory_policy: right_content.viewer_offer_enhance_directory_policy,
        show_enhance_zahir: right_content.viewer_offer_enhance_zahir,
    });
    true
}

fn try_open_lens_panel_space_menu(state: &mut UblxState, view: &ViewData) -> bool {
    if state.main_mode != MainMode::Lenses || view.filtered_categories.is_empty() {
        return false;
    }
    let Some(lens_name) = view
        .filtered_categories
        .get(state.panels.category_state.selected().unwrap_or(0))
        .cloned()
    else {
        return false;
    };
    state.open_space_menu(SpaceMenuKind::LensPanelActions { lens_name });
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
    if space_menu_open_blocked(state) {
        return false;
    }
    if state.panels.focus == PanelFocus::Contents {
        return try_open_file_space_menu(state, right_content);
    }
    try_open_lens_panel_space_menu(state, view)
}
