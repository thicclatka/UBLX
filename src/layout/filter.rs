//! Pure filtering for snapshot view: categories and content indices by search and category.
//! No state mutation; used by the event loop to build [super::setup::ViewData].

use super::setup::TuiRow;

/// Categories that have at least one row matching the search query.
/// If `search_query` is empty, returns all categories.
pub fn categories_for_search(
    categories: &[String],
    all_rows: &[TuiRow],
    search_query: &str,
) -> Vec<String> {
    let q = search_query.trim();
    if q.is_empty() {
        return categories.to_vec();
    }
    categories
        .iter()
        .filter(|cat| {
            all_rows
                .iter()
                .any(|(path, c, _)| c == *cat && (path.contains(q) || c.contains(q)))
        })
        .cloned()
        .collect()
}

/// Indices into `all_rows` for the current category and search (no row copy).
/// When `selected_category` is `None`, all rows are considered; otherwise only rows in that category.
/// Search filter is applied when `search_query` is non-empty.
pub fn content_indices_for_view(
    all_rows: &[TuiRow],
    selected_category: Option<&str>,
    search_query: &str,
) -> Vec<usize> {
    let q = search_query.trim();
    let match_search =
        |path: &str, category: &str| q.is_empty() || path.contains(q) || category.contains(q);

    match selected_category {
        None => all_rows
            .iter()
            .enumerate()
            .filter(|(_, (path, category, _))| match_search(path, category))
            .map(|(i, _)| i)
            .collect(),
        Some(cat) => all_rows
            .iter()
            .enumerate()
            .filter(|(_, (_, c, _))| c.as_str() == cat)
            .filter(|(_, (path, category, _))| match_search(path, category))
            .map(|(i, _)| i)
            .collect(),
    }
}

/// Filter raw delta rows (created_ns, path) by path containing query. Keeps dates when building display lines.
pub fn filter_delta_rows(rows: &[(i64, String)], search_query: &str) -> Vec<(i64, String)> {
    let q = search_query.trim();
    if q.is_empty() {
        return rows.to_vec();
    }
    rows.iter()
        .filter(|(_, path)| path.contains(q))
        .cloned()
        .collect()
}
