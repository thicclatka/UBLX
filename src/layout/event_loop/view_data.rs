//! Snapshot-mode view data: filtered categories and contents from search + selection.

use crate::layout::{filter, setup};
use crate::utils::{clamp_selection, clamp_selection_opt};

/// Compute filtered categories and contents from search + category selection; clamp list
/// selection and reset preview scroll when selection changes.
pub fn build_view_data(
    state: &mut setup::UblxState,
    categories: &[String],
    all_rows: &[setup::TuiRow],
) -> setup::ViewData {
    let search_query = state.search_query.trim();
    let filtered_categories = filter::categories_for_search(categories, all_rows, search_query);
    let category_idx = state.category_state.selected().unwrap_or(0);
    let selected_category = if category_idx == 0 {
        None
    } else {
        filtered_categories
            .get(category_idx - 1)
            .map(String::as_str)
    };
    let contents_indices =
        filter::content_indices_for_view(all_rows, selected_category, search_query);
    let category_list_len = 1 + filtered_categories.len();
    if category_list_len > 0 {
        state
            .category_state
            .select(Some(clamp_selection(category_idx, category_list_len)));
    }
    let content_len = contents_indices.len();
    if let Some(sel) = clamp_selection_opt(
        state.content_state.selected().unwrap_or(0),
        content_len,
    ) {
        state.content_state.select(Some(sel));
    } else {
        state.content_state.select(None);
    }
    let content_sel = state.content_state.selected();
    let preview_key = (category_idx, content_sel);
    if state.prev_preview_key.as_ref() != Some(&preview_key) {
        state.preview_scroll = 0;
        state.prev_preview_key = Some(preview_key);
    }
    setup::ViewData {
        filtered_categories,
        contents: setup::ViewContents::SnapshotIndices(contents_indices),
        category_list_len,
        content_len,
    }
}
