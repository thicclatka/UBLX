//! Pure filtering for snapshot view: categories and content indices by search and category.
//! No state mutation; used by the event loop to build [`super::setup::ViewData`].

use rayon::prelude::*;

use crate::config::PARALLEL;
use super::setup::TuiRow;

/// Categories that have at least one row matching the search query.
/// If `search_query` is empty, returns all categories.
/// Uses parallel iteration when there are many categories (≥ [`PARALLEL.categories_for_search`]) and search is non-empty.
#[must_use]
pub fn categories_for_search(
    categories: &[String],
    all_rows: &[TuiRow],
    search_query: &str,
) -> Vec<String> {
    let q = search_query.trim();
    if q.is_empty() {
        return categories.to_vec();
    }
    let matches = |cat: &String| {
        all_rows
            .iter()
            .any(|(path, c, _)| c == cat && (path.contains(q) || c.contains(q)))
    };
    if categories.len() >= PARALLEL.categories_for_search {
        categories.par_iter().filter(|cat| matches(cat)).cloned().collect()
    } else {
        categories.iter().filter(|cat| matches(cat)).cloned().collect()
    }
}

/// Indices into `all_rows` for the current category and search (no row copy).
/// When `selected_category` is `None`, all rows are considered; otherwise only rows in that category.
/// Search filter is applied when `search_query` is non-empty.
/// Uses parallel iteration when `all_rows.len()` exceeds [`PARALLEL.content_indices`].
#[must_use]
pub fn content_indices_for_view(
    all_rows: &[TuiRow],
    selected_category: Option<&str>,
    search_query: &str,
) -> Vec<usize> {
    let q = search_query.trim();
    let match_search =
        |path: &str, category: &str| q.is_empty() || path.contains(q) || category.contains(q);
    let match_row = |row: &TuiRow| {
        let (path, category, _) = row;
        selected_category.is_none_or(|c| category.as_str() == c) && match_search(path, category)
    };

    if all_rows.len() >= PARALLEL.content_indices {
        all_rows
            .par_iter()
            .enumerate()
            .filter(|(_, row)| match_row(row))
            .map(|(i, _)| i)
            .collect()
    } else {
        all_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| match_row(row))
            .map(|(i, _)| i)
            .collect()
    }
}

/// Filter raw delta rows (`created_ns`, path) by path containing query. Keeps dates when building display lines.
/// Uses parallel iteration when row count exceeds [`PARALLEL.delta_rows`].
#[must_use]
pub fn filter_delta_rows(rows: &[(i64, String)], search_query: &str) -> Vec<(i64, String)> {
    let q = search_query.trim();
    if q.is_empty() {
        return rows.to_vec();
    }
    if rows.len() >= PARALLEL.delta_rows {
        rows.par_iter()
            .filter(|(_, path)| path.contains(q))
            .cloned()
            .collect()
    } else {
        rows.iter()
            .filter(|(_, path)| path.contains(q))
            .cloned()
            .collect()
    }
}
