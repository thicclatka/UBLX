//! Spacebar context menu: row counts, labels, hotkeys, submit handling, and opening the menu.

use crate::app::RunUblxParams;
use crate::config::UblxOpts;
use crate::engine::db_ops;
use crate::layout::setup::{
    MainMode, PanelFocus, RightPaneContent, SpaceMenuKind, UblxState, ViewData,
};
use crate::modules;
use crate::ui::{UI_STRINGS, file_ops::modal_open, keymap::UblxAction, show_operation_toast};

#[must_use]
fn qa_menu_item_count(kind: Option<&SpaceMenuKind>, main_mode: MainMode) -> usize {
    match kind {
        Some(SpaceMenuKind::FileActions {
            show_enhance_directory_policy,
            show_enhance_zahir,
            show_copy_zahir_json,
            ..
        }) => {
            let n = 2 // Open, Show in folder
                + usize::from(*show_enhance_directory_policy)
                + usize::from(*show_enhance_zahir)
                + 1 // Add / Remove lens
                + 1 // Copy Path
                + usize::from(*show_copy_zahir_json)
                + 2; // Rename, Delete
            if main_mode == MainMode::Lenses {
                n - 1
            } else {
                n
            }
        }
        Some(
            SpaceMenuKind::LensPanelActions { .. } | SpaceMenuKind::DuplicateMemberActions { .. },
        ) => 2,
        None => 0,
    }
}

/// Indices: Open, Reveal, optional policy, optional Zahir, Lens, Copy Path, optional Copy JSON, Rename, Delete.
struct FileSpaceMenuIndices {
    open: usize,
    reveal: usize,
    policy: Option<usize>,
    zahir: Option<usize>,
    lens: usize,
    copy_path: usize,
    copy_json: Option<usize>,
    rename: usize,
    delete: usize,
}

/// `"{label} ({key})"` — same pattern as quick actions menu (spacebar) rows (e.g. bulk popup, lens actions).
#[must_use]
pub fn label_with_hotkey(label: &str, key: char) -> String {
    format!("{label} ({key})")
}

/// Menu labels in display order with `(letter)` hints. Must match [`qa_menu_hotkey_to_index`] and submit logic.
#[must_use]
pub fn qa_menu_item_labels(kind: &SpaceMenuKind, main_mode: MainMode) -> Vec<String> {
    match kind {
        SpaceMenuKind::FileActions {
            show_enhance_directory_policy,
            show_enhance_zahir,
            show_copy_zahir_json,
            ..
        } => {
            let mut v = vec![
                label_with_hotkey(UI_STRINGS.space.open, 'o'),
                label_with_hotkey(UI_STRINGS.space.show_in_folder, 'f'),
            ];
            if *show_enhance_directory_policy {
                v.push(label_with_hotkey(UI_STRINGS.space.enhance_policy, 'p'));
            }
            if *show_enhance_zahir {
                v.push(label_with_hotkey(
                    UI_STRINGS.space.enhance_with_zahirscan,
                    'z',
                ));
            }
            if main_mode == MainMode::Lenses {
                v.push(label_with_hotkey(UI_STRINGS.space.remove_from_lens, 'd'));
            } else {
                v.push(label_with_hotkey(UI_STRINGS.space.add_to_lens, 'l'));
            }
            v.push(label_with_hotkey(UI_STRINGS.space.copy_path, 'c'));
            if *show_copy_zahir_json {
                v.push(label_with_hotkey(UI_STRINGS.space.copy_zahir_json, 'j'));
            }
            v.push(label_with_hotkey(UI_STRINGS.space.rename, 'r'));
            if main_mode != MainMode::Lenses {
                v.push(label_with_hotkey(UI_STRINGS.space.delete, 'd'));
            }
            v
        }
        SpaceMenuKind::LensPanelActions { .. } => vec![
            label_with_hotkey(UI_STRINGS.space.rename, 'r'),
            label_with_hotkey(UI_STRINGS.space.delete, 'd'),
        ],
        SpaceMenuKind::DuplicateMemberActions { .. } => vec![
            label_with_hotkey(UI_STRINGS.space.delete, 'd'),
            label_with_hotkey(UI_STRINGS.space.ignore_in_duplicates, 'i'),
        ],
    }
}

/// Map a typed letter to a row index for the **current** menu (`None` if that row is not shown or key unknown).
#[must_use]
pub fn qa_menu_hotkey_to_index(
    kind: &SpaceMenuKind,
    key: char,
    main_mode: MainMode,
) -> Option<usize> {
    let c = key.to_ascii_lowercase();
    match kind {
        SpaceMenuKind::FileActions {
            show_enhance_directory_policy,
            show_enhance_zahir,
            show_copy_zahir_json,
            ..
        } => {
            let m = file_qa_menu_indices(
                *show_enhance_directory_policy,
                *show_enhance_zahir,
                *show_copy_zahir_json,
            );
            match c {
                'o' => Some(m.open),
                'f' => Some(m.reveal),
                'p' => m.policy,
                'z' => m.zahir,
                'l' => (main_mode != MainMode::Lenses).then_some(m.lens),
                'c' => Some(m.copy_path),
                'j' => m.copy_json,
                'r' => Some(m.rename),
                'd' => {
                    if main_mode == MainMode::Lenses {
                        Some(m.lens)
                    } else {
                        Some(m.delete)
                    }
                }
                _ => None,
            }
        }
        SpaceMenuKind::LensPanelActions { .. } => match c {
            'r' => Some(0),
            'd' => Some(1),
            _ => None,
        },
        SpaceMenuKind::DuplicateMemberActions { .. } => match c {
            'd' => Some(0),
            'i' => Some(1),
            _ => None,
        },
    }
}

fn file_qa_menu_indices(
    show_enhance_directory_policy: bool,
    show_enhance_zahir: bool,
    show_copy_zahir_json: bool,
) -> FileSpaceMenuIndices {
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
    i += 1;
    let copy_path = i;
    i += 1;
    let copy_json = show_copy_zahir_json.then(|| {
        let j = i;
        i += 1;
        j
    });
    let rename = i;
    i += 1;
    let delete = i;
    FileSpaceMenuIndices {
        open,
        reveal,
        policy,
        zahir,
        lens,
        copy_path,
        copy_json,
        rename,
        delete,
    }
}

fn qa_menu_enhance_zahir_if_disabled(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    path: &str,
    ublx_opts: &UblxOpts,
) {
    if ublx_opts.enable_enhance_all {
        return;
    }
    match modules::enhancer::enhance_single_path(
        &params.dir_to_ublx,
        &params.db_path,
        path,
        ublx_opts,
    ) {
        Ok(()) => {
            state.session.reload.snapshot_rows = true;
            show_operation_toast(
                state,
                params,
                UI_STRINGS.toasts.enhanced_with_zahirscan,
                "enhance",
                log::Level::Info,
            );
        }
        Err(e) => {
            log::warn!("Enhance with ZahirScan: {e}");
            show_operation_toast(
                state,
                params,
                format!("{}{e}", UI_STRINGS.toasts.enhance_failed_prefix),
                "enhance",
                log::Level::Info,
            );
        }
    }
}

/// Copy selected relative path as an absolute path using cached clipboard command;
/// emits a success/failure operation toast.
fn copy_zahir_json_to_clipboard(state: &mut UblxState, params: &RunUblxParams<'_>, path: &str) {
    let Some(json) = db_ops::load_zahir_json_for_path(&params.db_path, path)
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
    else {
        show_operation_toast(
            state,
            params,
            UI_STRINGS.toasts.copy_zahir_json_failed_prefix.to_string()
                + "no Zahir JSON for this path",
            "space",
            log::Level::Warn,
        );
        return;
    };
    let copied = state
        .clipboard_copy
        .as_ref()
        .ok_or_else(|| std::io::Error::other("no clipboard command detected"))
        .and_then(|cmd| cmd.copy_utf8(&json));
    match copied {
        Ok(()) => show_operation_toast(
            state,
            params,
            UI_STRINGS.toasts.copied_zahir_json_to_clipboard,
            "space",
            log::Level::Info,
        ),
        Err(e) => show_operation_toast(
            state,
            params,
            format!("{}{e}", UI_STRINGS.toasts.copy_zahir_json_failed_prefix),
            "space",
            log::Level::Warn,
        ),
    }
}

fn copy_selected_path_to_clipboard(state: &mut UblxState, params: &RunUblxParams<'_>, path: &str) {
    let full = params.dir_to_ublx.join(path);
    let copied = state
        .clipboard_copy
        .as_ref()
        .ok_or_else(|| std::io::Error::other("no clipboard command detected"))
        .and_then(|cmd| cmd.copy_utf8(&full.to_string_lossy()));
    match copied {
        Ok(()) => show_operation_toast(
            state,
            params,
            UI_STRINGS.toasts.copied_path_to_clipboard,
            "space",
            log::Level::Info,
        ),
        Err(e) => show_operation_toast(
            state,
            params,
            format!("{}{e}", UI_STRINGS.toasts.copy_path_failed_prefix),
            "space",
            log::Level::Warn,
        ),
    }
}

fn qa_menu_file_actions_submit(
    state: &mut UblxState,
    view: &ViewData,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    kind: SpaceMenuKind,
    idx: usize,
) {
    let SpaceMenuKind::FileActions {
        path,
        can_open_in_terminal,
        show_enhance_directory_policy,
        show_enhance_zahir,
        show_copy_zahir_json,
    } = kind
    else {
        return;
    };

    let m = file_qa_menu_indices(
        show_enhance_directory_policy,
        show_enhance_zahir,
        show_copy_zahir_json,
    );

    if idx == m.open {
        state.open_open_menu(path, can_open_in_terminal);
        return;
    }
    if idx == m.reveal {
        let full = params.dir_to_ublx.join(&path);
        if let Err(e) = modules::opener::reveal_in_file_manager(&full) {
            log::warn!("Show in folder: {e}");
        }
        return;
    }
    if m.policy == Some(idx) {
        state.enhance_policy_menu.visible = true;
        state.enhance_policy_menu.path = Some(path);
        state.enhance_policy_menu.selected_index = 0;
        return;
    }
    if m.zahir == Some(idx) {
        qa_menu_enhance_zahir_if_disabled(state, params, &path, ublx_opts);
        return;
    }
    if idx == m.lens {
        if state.main_mode == MainMode::Snapshot {
            state.open_lens_menu(vec![path], None);
            return;
        }
        let Some(lens_name) = view
            .filtered_categories
            .get(state.panels.category_state.selected().unwrap_or(0))
        else {
            return;
        };
        if modules::lenses::remove_path_from_lens(&params.db_path, lens_name, &path).is_ok() {
            show_operation_toast(
                state,
                params,
                UI_STRINGS
                    .toasts
                    .removed_from_lens
                    .replace("{LENS}", lens_name),
                "lens",
                log::Level::Info,
            );
        }
        return;
    }
    if idx == m.copy_path {
        copy_selected_path_to_clipboard(state, params, &path);
        return;
    }
    if m.copy_json == Some(idx) {
        copy_zahir_json_to_clipboard(state, params, &path);
        return;
    }
    if idx == m.rename {
        state.open_file_rename_input(path);
        return;
    }
    if idx == m.delete && state.main_mode != MainMode::Lenses {
        state.open_file_delete_confirm(path);
    }
}

fn duplicate_member_actions_submit(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    path: String,
    idx: usize,
) {
    match idx {
        0 => state.open_file_delete_confirm(path),
        1 => {
            state.duplicate_ignored_paths.insert(path);
            show_operation_toast(
                state,
                params,
                UI_STRINGS.toasts.duplicate_member_ignored,
                "duplicates",
                log::Level::Info,
            );
        }
        _ => {}
    }
}

fn qa_menu_apply_submit(
    state: &mut UblxState,
    view: &ViewData,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    kind: SpaceMenuKind,
    idx: usize,
) {
    match kind {
        fa @ SpaceMenuKind::FileActions { .. } => {
            qa_menu_file_actions_submit(state, view, params, ublx_opts, fa, idx);
        }
        SpaceMenuKind::DuplicateMemberActions { path } => {
            duplicate_member_actions_submit(state, params, path, idx);
        }
        SpaceMenuKind::LensPanelActions { lens_name } => match idx {
            0 => {
                state.lens_confirm.rename_input = Some((lens_name.clone(), lens_name));
            }
            _ => {
                state.open_lens_delete_confirm(lens_name);
            }
        },
    }
}

/// Handle action when spacebar context menu is visible. Returns true if handled.
pub fn handle_qa_menu(
    state: &mut UblxState,
    view: &ViewData,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    action: UblxAction,
) -> bool {
    if !state.qa_menu.visible {
        return false;
    }
    let item_count = qa_menu_item_count(state.qa_menu.kind.as_ref(), state.main_mode);
    match action {
        UblxAction::Quit | UblxAction::SearchClear => state.close_qa_menu(),
        UblxAction::MoveDown => {
            state.qa_menu.selected_index =
                (state.qa_menu.selected_index + 1).min(item_count.saturating_sub(1));
        }
        UblxAction::MoveUp => {
            state.qa_menu.selected_index = state.qa_menu.selected_index.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            let kind = state.qa_menu.kind.clone();
            let idx = state.qa_menu.selected_index;
            state.close_qa_menu();
            if let Some(k) = kind {
                qa_menu_apply_submit(state, view, params, ublx_opts, k, idx);
            }
        }
        UblxAction::SpaceMenuHotkeySelect(idx) if idx < item_count => {
            let kind = state.qa_menu.kind.clone();
            state.close_qa_menu();
            if let Some(k) = kind {
                qa_menu_apply_submit(state, view, params, ublx_opts, k, idx);
            }
        }
        _ => {}
    }
    true
}

#[must_use]
fn qa_menu_open_blocked(state: &UblxState) -> bool {
    !matches!(
        state.main_mode,
        MainMode::Snapshot | MainMode::Lenses | MainMode::Duplicates,
    ) || state.qa_menu.visible
        || state.enhance_policy_menu.visible
        || state.lens_confirm.rename_input.is_some()
        || state.lens_confirm.delete_visible
        || state.open_menu.visible
        || state.lens_menu.visible
        || state.lens_menu.name_input.is_some()
        || modal_open(state)
}

fn try_open_file_qa_menu(state_mut: &mut UblxState, right_content_ref: &RightPaneContent) -> bool {
    let Some(path) = right_content_ref.snap_meta.path.as_ref() else {
        return false;
    };
    if state_mut.main_mode == MainMode::Duplicates {
        state_mut.open_qa_menu(SpaceMenuKind::DuplicateMemberActions { path: path.clone() });
        return true;
    }
    state_mut.open_qa_menu(SpaceMenuKind::FileActions {
        path: path.clone(),
        can_open_in_terminal: right_content_ref.derived.can_open,
        show_enhance_directory_policy: right_content_ref.derived.offer_enhance_directory_policy,
        show_enhance_zahir: right_content_ref.derived.offer_enhance_zahir,
        show_copy_zahir_json: right_content_ref.snap_meta.has_zahir_json,
    });
    true
}

fn try_open_lens_panel_qa_menu(state_mut: &mut UblxState, view_ref: &ViewData) -> bool {
    if state_mut.main_mode != MainMode::Lenses || view_ref.filtered_categories.is_empty() {
        return false;
    }
    let Some(lens_name) = view_ref
        .filtered_categories
        .get(state_mut.panels.category_state.selected().unwrap_or(0))
        .cloned()
    else {
        return false;
    };
    state_mut.open_qa_menu(SpaceMenuKind::LensPanelActions { lens_name });
    true
}

/// If action is `SpaceMenu` and context allows, open the quick actions menu (spacebar). Returns true if opened.
pub fn try_open_qa_menu(
    state_mut: &mut UblxState,
    view_ref: &ViewData,
    right_content_ref: &RightPaneContent,
    action: UblxAction,
) -> bool {
    if !matches!(action, UblxAction::SpaceMenu) {
        return false;
    }
    if state_mut.main_mode == MainMode::Delta {
        return false;
    }
    if qa_menu_open_blocked(state_mut) {
        return false;
    }
    if state_mut.panels.focus == PanelFocus::Contents {
        return try_open_file_qa_menu(state_mut, right_content_ref);
    }
    try_open_lens_panel_qa_menu(state_mut, view_ref)
}
