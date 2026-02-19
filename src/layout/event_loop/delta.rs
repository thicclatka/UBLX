//! Delta-mode view data: overview, added/mod/removed rows, filtered display lines.

use crate::layout::{filter, setup};
use crate::ui::UI_STRINGS;
use crate::utils::format::format_timestamp_ns;

use crate::engine::db_ops;

/// Load delta_log data for Delta mode: overview text (snapshot count + timestamps) and paths per type.
pub fn build_delta_view_data(db_path: &std::path::Path) -> setup::DeltaViewData {
    let timestamps = db_ops::load_delta_log_snapshot_timestamps(db_path).unwrap_or_default();
    let snapshot_count = timestamps.len();
    let overview_lines: Vec<String> = std::iter::once(String::new())
        .chain(std::iter::once(format!(
            "{} snapshot(s) (sorted by time; newest first):",
            snapshot_count
        )))
        .chain(std::iter::once(String::new()))
        .chain(
            timestamps
                .into_iter()
                .map(|ns| format!("  • {}", format_timestamp_ns(ns))),
        )
        .collect();
    let overview_text = overview_lines.join("\n");

    let added_rows = db_ops::load_delta_log_rows_by_type(db_path, "added").unwrap_or_default();
    let mod_rows = db_ops::load_delta_log_rows_by_type(db_path, "mod").unwrap_or_default();
    let removed_rows = db_ops::load_delta_log_rows_by_type(db_path, "removed").unwrap_or_default();

    setup::DeltaViewData {
        overview_text,
        added_rows,
        mod_rows,
        removed_rows,
    }
}

fn build_delta_display_lines(rows: Vec<(i64, String)>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_ns: Option<i64> = None;
    for (ns, path) in rows {
        if current_ns != Some(ns) {
            current_ns = Some(ns);
            lines.push(format_timestamp_ns(ns));
        }
        lines.push(format!("  {}", path));
    }
    lines
}

/// Clamp list selection for Delta mode (category and content from view).
pub fn clamp_delta_selection(state: &mut setup::UblxState, view: &setup::ViewData) {
    let cat_max = view.category_list_len.saturating_sub(1);
    let cat_idx = state.category_state.selected().unwrap_or(0).min(cat_max);
    state.category_state.select(Some(cat_idx));
    let len = view.content_len;
    if len > 0 {
        let sel = state
            .content_state
            .selected()
            .unwrap_or(0)
            .min(len.saturating_sub(1));
        state.content_state.select(Some(sel));
    } else {
        state.content_state.select(None);
    }
}

/// Delta mode category names (order matches [setup::DeltaViewData::rows_by_index] 0, 1, 2).
const DELTA_CATEGORIES: &[&str] = &[
    UI_STRINGS.delta_added,
    UI_STRINGS.delta_mod,
    UI_STRINGS.delta_removed,
];

/// ViewData for Delta mode. Search filters by path; display lines keep timestamp groupings (dates preserved).
pub fn view_data_for_delta_mode(
    state: &setup::UblxState,
    delta: &setup::DeltaViewData,
) -> setup::ViewData {
    let search_query = state.search_query.trim();
    let cat_idx = state
        .category_state
        .selected()
        .unwrap_or(0)
        .min(DELTA_CATEGORIES.len().saturating_sub(1));
    let raw_rows = delta.rows_by_index(cat_idx);
    let filtered_rows = filter::filter_delta_rows(raw_rows, search_query);
    let display_lines = build_delta_display_lines(filtered_rows);
    let content_len = display_lines.len();
    let rows: Vec<setup::TuiRow> = display_lines
        .into_iter()
        .map(|line| (line, String::new(), 0u64))
        .collect();
    setup::ViewData {
        filtered_categories: DELTA_CATEGORIES.iter().map(|s| (*s).to_string()).collect(),
        contents: setup::ViewContents::DeltaRows(rows),
        category_list_len: DELTA_CATEGORIES.len(),
        content_len,
    }
}
