//! Duplicates-mode view data: left = one name per duplicate group, middle = paths in that group, right = viewer for selected path.

use crate::engine::db_ops::DuplicateGroup;
use crate::layout::setup;
use crate::utils::format::{clamp_selection, clamp_selection_opt};

/// ViewData for Duplicates mode. Categories = duplicate group names (filtered by search); contents = paths in selected group.
pub fn view_data_for_duplicates_mode(
    state: &setup::UblxState,
    groups: &[DuplicateGroup],
) -> setup::ViewData {
    let search_query = state.search_query.trim();
    let filtered_names: Vec<String> = if search_query.is_empty() {
        groups
            .iter()
            .map(|g| g.representative_name().to_string())
            .collect()
    } else {
        groups
            .iter()
            .filter(|g| {
                g.representative_name().contains(search_query)
                    || g.paths.iter().any(|p| p.contains(search_query))
            })
            .map(|g| g.representative_name().to_string())
            .collect()
    };
    let category_list_len = filtered_names.len();
    let cat_idx = clamp_selection(
        state.category_state.selected().unwrap_or(0),
        category_list_len.max(1),
    );
    let selected_paths: Vec<String> = filtered_names
        .get(cat_idx)
        .and_then(|name| groups.iter().find(|g| g.representative_name() == name))
        .map(|g| g.paths.clone())
        .unwrap_or_default();
    let path_rows: Vec<setup::TuiRow> = selected_paths
        .iter()
        .map(|p| (p.clone(), String::new(), 0u64))
        .collect();
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
        category_list_len: category_list_len.max(1),
        content_len,
    }
}

/// Clamp list selection for Duplicates mode.
pub fn clamp_duplicates_selection(state: &mut setup::UblxState, view: &setup::ViewData) {
    let cat_idx = clamp_selection(
        state.category_state.selected().unwrap_or(0),
        view.category_list_len,
    );
    state.category_state.select(Some(cat_idx));
    let len = view.content_len;
    if let Some(sel) = clamp_selection_opt(state.content_state.selected().unwrap_or(0), len) {
        state.content_state.select(Some(sel));
    } else {
        state.content_state.select(None);
    }
}
