//! Snapshot-mode view data: filtered categories and contents from search + selection.

use crate::layout::{filter, setup};
use crate::utils::{clamp_selection, clamp_selection_opt};

/// Resolve the selected category string from the category list index (0 = "All").
fn selected_category(filtered_categories: &[String], category_idx: usize) -> Option<&str> {
    if category_idx == 0 {
        None
    } else {
        filtered_categories
            .get(category_idx - 1)
            .map(String::as_str)
    }
}

/// Clamp category list selection to valid range and update state.
fn clamp_category_selection(state: &mut setup::UblxState, category_list_len: usize) {
    if category_list_len == 0 {
        return;
    }
    let category_idx = state.panels.category_state.selected().unwrap_or(0);
    state
        .panels
        .category_state
        .select(Some(clamp_selection(category_idx, category_list_len)));
}

/// Clamp content list selection to valid range and update state.
fn clamp_content_selection(state: &mut setup::UblxState, content_len: usize) {
    let current = state.panels.content_state.selected().unwrap_or(0);
    if let Some(sel) = clamp_selection_opt(current, content_len) {
        state.panels.content_state.select(Some(sel));
    } else {
        state.panels.content_state.select(None);
    }
}

/// Reset preview scroll when category or content selection changes.
fn sync_preview_scroll(state: &mut setup::UblxState, category_idx: usize) {
    let content_sel = state.panels.content_state.selected();
    let preview_key = (category_idx, content_sel);
    if state.panels.prev_preview_key.as_ref() != Some(&preview_key) {
        state.panels.preview_scroll = 0;
        state.panels.prev_preview_key = Some(preview_key);
    }
}

/// Compute filtered categories and contents from search + category selection; clamp list
/// selection and reset preview scroll when selection changes.
pub fn build_view_data(
    state: &mut setup::UblxState,
    categories: &[String],
    all_rows: &[setup::TuiRow],
) -> setup::ViewData {
    let search_query = state.search.query.trim();
    let filtered_categories = filter::categories_for_search(categories, all_rows, search_query);
    let category_idx = state.panels.category_state.selected().unwrap_or(0);
    let selected_category = selected_category(&filtered_categories, category_idx);
    let contents_indices =
        filter::content_indices_for_view(all_rows, selected_category, search_query);

    let category_list_len = 1 + filtered_categories.len();
    let content_len = contents_indices.len();

    clamp_category_selection(state, category_list_len);
    clamp_content_selection(state, content_len);
    sync_preview_scroll(state, category_idx);

    setup::ViewData {
        filtered_categories,
        contents: setup::ViewContents::SnapshotIndices(contents_indices),
        category_list_len,
        content_len,
    }
}
