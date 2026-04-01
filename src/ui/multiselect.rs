//! Middle-pane multi-select: **Ctrl+Space** toggles mode (contents pane only; Snapshot & Lenses, not Duplicates); **Space** toggles rows; **a** opens bulk menu (**r** = bulk rename via `$EDITOR`, **a** = add-to-lens, **d** = delete / delete-from-lens; **z** = Enhance with `ZahirScan` when eligible).

use std::collections::HashSet;

use crate::app::RunUblxParams;
use crate::config::UblxOpts;
use crate::engine::db_ops;
use crate::layout::setup::{
    CATEGORY_DIRECTORY, MainMode, MultiselectState, PanelFocus, TuiRow, UblxState, ViewData,
};
use crate::modules::{enhance, lens as lens_applet};
use crate::ui::{UI_STRINGS, file_ops, keymap::UblxAction, show_operation_toast};

fn multiselect_applies(main_mode: MainMode) -> bool {
    matches!(main_mode, MainMode::Snapshot | MainMode::Lenses)
}

fn rel_path_at(view: &ViewData, all_rows: Option<&[TuiRow]>, state: &UblxState) -> Option<String> {
    let i = state.panels.content_state.selected()?;
    view.row_at(i, all_rows).map(|row| row.0.clone())
}

/// Selected paths in middle-pane list order (stable; not `HashSet` iteration order).
fn selected_paths_in_list_order(
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    selected: &HashSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..view.content_len {
        if let Some(row) = view.row_at(i, all_rows)
            && selected.contains(&row.0)
        {
            out.push(row.0.clone());
        }
    }
    out
}

fn category_for_path<'a>(
    view: &'a ViewData,
    all_rows: Option<&'a [TuiRow]>,
    path: &str,
) -> Option<&'a str> {
    for i in 0..view.content_len {
        let row = view.row_at(i, all_rows)?;
        if row.0 == path {
            return Some(row.1.as_str());
        }
    }
    None
}

/// True when the bulk menu should show **Enhance with `ZahirScan`** (non-directory rows with empty `zahir_json`, `enable_enhance_all` off).
fn bulk_zahir_row_visible(
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    selected: &HashSet<String>,
    db_path: &std::path::Path,
    ublx_opts: &UblxOpts,
) -> bool {
    if ublx_opts.enable_enhance_all {
        return false;
    }
    for i in 0..view.content_len {
        let Some(row) = view.row_at(i, all_rows) else {
            continue;
        };
        if !selected.contains(&row.0) {
            continue;
        }
        if row.1 == CATEGORY_DIRECTORY {
            continue;
        }
        let zahir_json = db_ops::load_zahir_json_for_path(db_path, &row.0)
            .ok()
            .flatten()
            .unwrap_or_default();
        if zahir_json.is_empty() {
            return true;
        }
    }
    false
}

#[must_use]
fn bulk_menu_max_index(ms: &MultiselectState) -> usize {
    if ms.bulk_menu_zahir_row { 3 } else { 2 }
}

/// Toggle multi-select with **Ctrl+Space**. Consumes the key only when the contents pane is focused.
/// Turning mode on inserts the current content row into the selection.
pub fn try_toggle_mode(
    state: &mut UblxState,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
) -> bool {
    if !multiselect_applies(state.main_mode) {
        return false;
    }
    if !matches!(state.panels.focus, PanelFocus::Contents) {
        return false;
    }
    state.multiselect.active = !state.multiselect.active;
    if state.multiselect.active {
        state.multiselect.bulk_menu_visible = false;
        state.multiselect.bulk_menu_selected = 0;
        state.multiselect.bulk_menu_zahir_row = false;
        if let Some(rel) = rel_path_at(view, all_rows, state) {
            state.multiselect.selected.insert(rel);
        }
    } else {
        state.multiselect.clear();
    }
    true
}

fn set_bulk_menu_open(
    state: &mut UblxState,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    params: &RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
) {
    if state.multiselect.selected.is_empty() {
        return;
    }
    state.multiselect.bulk_menu_visible = true;
    state.multiselect.bulk_menu_selected = 0;
    state.multiselect.bulk_menu_zahir_row = bulk_zahir_row_visible(
        view,
        all_rows,
        &state.multiselect.selected,
        &params.db_path,
        ublx_opts,
    );
}

fn close_bulk_menu(state: &mut MultiselectState) {
    state.bulk_menu_visible = false;
    state.bulk_menu_selected = 0;
    state.bulk_menu_zahir_row = false;
}

fn run_bulk_enhance_zahir(
    state: &mut UblxState,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    paths: &[String],
) {
    if ublx_opts.enable_enhance_all {
        return;
    }
    let mut ok = 0usize;
    let mut failed = 0usize;
    for path in paths {
        let Some(cat) = category_for_path(view, all_rows, path) else {
            continue;
        };
        if cat == CATEGORY_DIRECTORY {
            continue;
        }
        let zahir_json = db_ops::load_zahir_json_for_path(&params.db_path, path)
            .ok()
            .flatten()
            .unwrap_or_default();
        if !zahir_json.is_empty() {
            continue;
        }
        match enhance::enhance_single_path(&params.dir_to_ublx, &params.db_path, path, ublx_opts) {
            Ok(()) => ok += 1,
            Err(e) => {
                failed += 1;
                log::warn!("Bulk Enhance with ZahirScan ({path}): {e}");
            }
        }
    }
    if ok > 0 {
        state.session.reload.snapshot_rows = true;
        let msg = UI_STRINGS
            .toasts
            .bulk_enhanced_zahir_n
            .replace("{N}", &ok.to_string());
        show_operation_toast(state, params, msg, "enhance", log::Level::Info);
    }
    if failed > 0 && ok == 0 {
        show_operation_toast(
            state,
            params,
            format!(
                "{}{failed} file(s) failed",
                UI_STRINGS.toasts.enhance_failed_prefix
            ),
            "enhance",
            log::Level::Info,
        );
    }
}

fn run_bulk_action(
    state: &mut UblxState,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    index: usize,
) {
    let paths = selected_paths_in_list_order(view, all_rows, &state.multiselect.selected);
    let mode = state.main_mode;
    let max_i = bulk_menu_max_index(&state.multiselect);
    close_bulk_menu(&mut state.multiselect);
    if index > max_i {
        return;
    }
    if mode == MainMode::Lenses {
        match index {
            0 => {
                if !paths.is_empty() {
                    file_ops::bulk_rename_via_editor(state, params, &paths, ublx_opts);
                }
            }
            1 => {
                if paths.is_empty() {
                    return;
                }
                let ex = view
                    .filtered_categories
                    .get(state.panels.category_state.selected().unwrap_or(0))
                    .cloned();
                state.open_lens_menu(paths, ex);
            }
            2 => {
                let Some(lens_name) = view
                    .filtered_categories
                    .get(state.panels.category_state.selected().unwrap_or(0))
                else {
                    return;
                };
                let mut ok = 0usize;
                for path in &paths {
                    if lens_applet::remove_path_from_lens(&params.db_path, lens_name, path).is_ok()
                    {
                        ok += 1;
                    }
                }
                if ok > 0 {
                    let msg = UI_STRINGS
                        .toasts
                        .bulk_removed_n_from_lens
                        .replace("{N}", &ok.to_string())
                        .replace("{LENS}", lens_name);
                    show_operation_toast(state, params, msg, "lens", log::Level::Info);
                }
            }
            3 => {
                run_bulk_enhance_zahir(state, view, all_rows, params, ublx_opts, &paths);
            }
            _ => {}
        }
        return;
    }

    match index {
        0 => {
            if !paths.is_empty() {
                file_ops::bulk_rename_via_editor(state, params, &paths, ublx_opts);
            }
        }
        1 => {
            if !paths.is_empty() {
                state.open_lens_menu(paths, None);
            }
        }
        2 => {
            state.open_file_delete_confirm_bulk(paths);
        }
        3 => {
            run_bulk_enhance_zahir(state, view, all_rows, params, ublx_opts, &paths);
        }
        _ => {}
    }
}

/// Handle multi-select and bulk menu keys. Returns true when the event is fully handled (skip default action application).
pub fn handle_key(
    state: &mut UblxState,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    action: UblxAction,
) -> bool {
    if file_ops::modal_open(state) {
        return false;
    }
    if state.multiselect.bulk_menu_visible {
        return handle_bulk_menu(state, view, all_rows, params, ublx_opts, action);
    }

    if !state.multiselect.active {
        return false;
    }

    match action {
        UblxAction::MultiselectToggleRow => {
            if !matches!(state.panels.focus, PanelFocus::Contents) {
                return true;
            }
            let Some(rel) = rel_path_at(view, all_rows, state) else {
                return true;
            };
            if state.multiselect.selected.remove(&rel) {
            } else {
                state.multiselect.selected.insert(rel);
            }
            true
        }
        UblxAction::MultiselectOpenBulkMenu => {
            if !matches!(state.panels.focus, PanelFocus::Contents) {
                return true;
            }
            if state.multiselect.selected.is_empty() {
                show_operation_toast(
                    state,
                    params,
                    UI_STRINGS.toasts.multiselect_none_selected,
                    "multiselect",
                    log::Level::Info,
                );
            } else {
                set_bulk_menu_open(state, view, all_rows, params, ublx_opts);
            }
            true
        }
        UblxAction::BulkMenuHotkeySelect(i) => {
            let max_i = bulk_menu_max_index(&state.multiselect);
            if i <= max_i {
                run_bulk_action(state, view, all_rows, params, ublx_opts, i);
            }
            true
        }
        UblxAction::MultiselectCancel => {
            state.multiselect.clear();
            true
        }
        _ => false,
    }
}

fn handle_bulk_menu(
    state: &mut UblxState,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
    action: UblxAction,
) -> bool {
    let max_i = bulk_menu_max_index(&state.multiselect);
    match action {
        UblxAction::Quit | UblxAction::SearchClear => {
            close_bulk_menu(&mut state.multiselect);
        }
        UblxAction::MoveDown => {
            state.multiselect.bulk_menu_selected =
                (state.multiselect.bulk_menu_selected + 1).min(max_i);
        }
        UblxAction::MoveUp => {
            state.multiselect.bulk_menu_selected =
                state.multiselect.bulk_menu_selected.saturating_sub(1);
        }
        UblxAction::SearchSubmit => {
            let i = state.multiselect.bulk_menu_selected;
            run_bulk_action(state, view, all_rows, params, ublx_opts, i);
        }
        UblxAction::BulkMenuHotkeySelect(i) => {
            if i <= max_i {
                run_bulk_action(state, view, all_rows, params, ublx_opts, i);
            }
        }
        _ => {}
    }
    true
}
