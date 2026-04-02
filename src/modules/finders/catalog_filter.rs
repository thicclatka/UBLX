//! Catalog search filtering: fuzzy match on snapshot paths/categories, delta paths, and related lists.
//! Pure functions; no state mutation. Callers build [`ViewData`] / delta display lines.

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;

use crate::config::PARALLEL;
use crate::layout::setup::TuiRow;

#[inline]
fn trim_q(q: &str) -> &str {
    q.trim()
}

/// Best fuzzy score for matching `needle` against `path` or `category` (higher is better).
#[must_use]
pub fn row_fuzzy_score(path: &str, category: &str, needle: &str) -> Option<i64> {
    let needle_trim = trim_q(needle);
    if needle_trim.is_empty() {
        return None;
    }
    let matcher = SkimMatcherV2::default();
    let path_score = matcher.fuzzy_match(path, needle_trim);
    let category_score = matcher.fuzzy_match(category, needle_trim);
    match (path_score, category_score) {
        (Some(path_pts), Some(cat_pts)) => Some(path_pts.max(cat_pts)),
        (Some(path_pts), None) => Some(path_pts),
        (None, Some(cat_pts)) => Some(cat_pts),
        (None, None) => None,
    }
}

/// True when `haystack` fuzzy-matches `needle` (empty needle matches nothing here — callers gate on empty).
#[must_use]
pub fn fuzzy_matches_field(haystack: &str, needle: &str) -> bool {
    let q = trim_q(needle);
    if q.is_empty() {
        return true;
    }
    SkimMatcherV2::default().fuzzy_match(haystack, q).is_some()
}

/// Categories that have at least one row matching the search query (fuzzy on path or category string),
/// or whose category label fuzzy-matches the query.
/// If `search_query` is empty, returns all categories.
/// Uses parallel iteration when there are many categories (≥ [`PARALLEL.categories_for_search`]) and search is non-empty.
#[must_use]
pub fn categories_for_search(
    categories: &[String],
    all_rows: &[TuiRow],
    search_query: &str,
) -> Vec<String> {
    let q = trim_q(search_query);
    if q.is_empty() {
        return categories.to_vec();
    }
    let matches = |cat: &String| {
        fuzzy_matches_field(cat, q)
            || all_rows
                .iter()
                .any(|(path, c, _)| c == cat && row_fuzzy_score(path, c, q).is_some())
    };
    if categories.len() >= PARALLEL.categories_for_search {
        categories
            .par_iter()
            .filter(|cat| matches(cat))
            .cloned()
            .collect()
    } else {
        categories
            .iter()
            .filter(|cat| matches(cat))
            .cloned()
            .collect()
    }
}

/// Indices into `all_rows` for the current category and search (no row copy).
/// When `selected_category` is `None`, all rows are considered; otherwise only rows in that category.
/// Non-empty search: only fuzzy-matching rows, **ordered by score (desc) then path (asc)**.
/// Empty search: all rows in category (unordered — caller applies sort mode).
/// Uses parallel iteration when `all_rows.len()` exceeds [`PARALLEL.content_indices`].
#[must_use]
pub fn content_indices_for_view(
    all_rows: &[TuiRow],
    selected_category: Option<&str>,
    search_query: &str,
) -> Vec<usize> {
    let q = trim_q(search_query);
    let in_category = |row: &TuiRow| selected_category.is_none_or(|c| row.1.as_str() == c);

    if q.is_empty() {
        let pick = |row: &TuiRow| in_category(row);
        if all_rows.len() >= PARALLEL.content_indices {
            return all_rows
                .par_iter()
                .enumerate()
                .filter(|(_, row)| pick(row))
                .map(|(i, _)| i)
                .collect();
        }
        return all_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| pick(row))
            .map(|(i, _)| i)
            .collect();
    }

    let mut scored: Vec<(usize, i64, String)> = if all_rows.len() >= PARALLEL.content_indices {
        all_rows
            .par_iter()
            .enumerate()
            .filter_map(|(i, row)| {
                if !in_category(row) {
                    return None;
                }
                let (path, cat, _) = row;
                let score = row_fuzzy_score(path, cat, q)?;
                Some((i, score, path.clone()))
            })
            .collect()
    } else {
        all_rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                if !in_category(row) {
                    return None;
                }
                let (path, cat, _) = row;
                let score = row_fuzzy_score(path, cat, q)?;
                Some((i, score, path.clone()))
            })
            .collect()
    };

    scored.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.into_iter().map(|(i, _, _)| i).collect()
}

/// Filter raw delta rows (`created_ns`, path) by fuzzy path match. Keeps dates when building display lines.
/// Uses parallel iteration when row count exceeds [`PARALLEL.delta_rows`].
#[must_use]
pub fn filter_delta_rows(rows: &[(i64, String)], search_query: &str) -> Vec<(i64, String)> {
    let q = trim_q(search_query);
    if q.is_empty() {
        return rows.to_vec();
    }
    let matcher = SkimMatcherV2::default();
    if rows.len() >= PARALLEL.delta_rows {
        rows.par_iter()
            .filter(|(_, path)| matcher.fuzzy_match(path, q).is_some())
            .cloned()
            .collect()
    } else {
        rows.iter()
            .filter(|(_, path)| matcher.fuzzy_match(path, q).is_some())
            .cloned()
            .collect()
    }
}
