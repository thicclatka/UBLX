//! Lenses mode: left = lens names, middle = paths in selected lens, right = viewer for selected path.

use std::path::Path;

use crate::engine::db_ops;
use crate::layout::setup;
use crate::utils::format::{clamp_selection, clamp_selection_opt};

/// ViewData for Lenses mode. Categories = lens names (filtered by search); contents = paths in selected lens from DB.
pub fn view_data_for_lenses_mode(
    state: &setup::UblxState,
    lens_names: &[String],
    db_path: &Path,
) -> setup::ViewData {
    let search_query = state.search.query.trim();
    let filtered_names: Vec<String> = if search_query.is_empty() {
        lens_names.to_vec()
    } else {
        lens_names
            .iter()
            .filter(|n| n.contains(search_query))
            .cloned()
            .collect()
    };
    let category_list_len = filtered_names.len().max(1);
    let cat_idx = clamp_selection(
        state.panels.category_state.selected().unwrap_or(0),
        category_list_len,
    );
    let selected_lens_name = filtered_names.get(cat_idx).map(String::as_str);
    let path_rows: Vec<setup::TuiRow> = selected_lens_name
        .and_then(|name| db_ops::load_lens_paths(db_path, name).ok())
        .unwrap_or_default();
    let contents: Vec<setup::TuiRow> = if search_query.is_empty() {
        path_rows
    } else {
        path_rows
            .into_iter()
            .filter(|(path, _, _)| path.contains(search_query))
            .collect()
    };
    let content_len = contents.len();
    setup::ViewData {
        filtered_categories: filtered_names,
        contents: setup::ViewContents::DeltaRows(contents),
        category_list_len,
        content_len,
    }
}

/// Clamp list selection for Lenses mode.
pub fn clamp_lenses_selection(state: &mut setup::UblxState, view: &setup::ViewData) {
    let cat_idx = clamp_selection(
        state.panels.category_state.selected().unwrap_or(0),
        view.category_list_len,
    );
    state.panels.category_state.select(Some(cat_idx));
    let len = view.content_len;
    if let Some(sel) = clamp_selection_opt(state.panels.content_state.selected().unwrap_or(0), len) {
        state.panels.content_state.select(Some(sel));
    } else {
        state.panels.content_state.select(None);
    }
}
