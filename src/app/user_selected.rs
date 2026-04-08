//! User-selected list modes: Duplicates and Lenses. Same two-pane structure (categories + contents);
//! only the source of category names and content rows differs.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use crate::config::PARALLEL;
use crate::engine::db_ops::{self, DuplicateGroup};
use crate::layout::setup;
use crate::modules::catalog_filter::fuzzy_matches_field;
use crate::utils::clamp_selection;

use super::view_data;

/// Basename of the duplicate group's representative path (shortest member path).
fn basename_for_duplicate_group(g: &DuplicateGroup) -> String {
    let rep = g.representative_name();
    Path::new(rep)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(rep)
        .to_string()
}

/// Unique left-pane labels when several groups share the same basename (`name`, `name (2)`, …).
fn disambiguated_duplicate_labels(groups: &[&DuplicateGroup]) -> Vec<String> {
    let basenames: Vec<String> = groups.iter().map(|g| basename_for_duplicate_group(g)).collect();
    let mut count_per: HashMap<String, usize> = HashMap::new();
    for b in &basenames {
        *count_per.entry(b.clone()).or_insert(0) += 1;
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    basenames
        .into_iter()
        .map(|b| {
            let total = *count_per.get(&b).unwrap_or(&1);
            if total <= 1 {
                return b;
            }
            let n = seen.entry(b.clone()).or_insert(0);
            *n += 1;
            if *n == 1 {
                b
            } else {
                format!("{b} ({n})")
            }
        })
        .collect()
}

/// Groups that pass search, in list order (left pane labels align with middle pane rows).
fn duplicate_groups_matching_search<'a>(
    groups: &'a [DuplicateGroup],
    search_query: &str,
) -> Vec<&'a DuplicateGroup> {
    if search_query.is_empty() {
        groups.iter().collect()
    } else if groups.len() >= PARALLEL.user_selected_filter {
        groups
            .par_iter()
            .filter(|g| {
                fuzzy_matches_field(g.representative_name(), search_query)
                    || g.paths.iter().any(|p| fuzzy_matches_field(p, search_query))
            })
            .collect()
    } else {
        groups
            .iter()
            .filter(|g| {
                fuzzy_matches_field(g.representative_name(), search_query)
                    || g.paths.iter().any(|p| fuzzy_matches_field(p, search_query))
            })
            .collect()
    }
}

/// Drop ignored paths; remove groups with fewer than two paths left.
#[must_use]
pub fn filter_duplicate_groups_for_view(
    groups: &[DuplicateGroup],
    ignored: &HashSet<String>,
) -> Vec<DuplicateGroup> {
    let mut out = Vec::new();
    for g in groups {
        let paths: Vec<String> = g
            .paths
            .iter()
            .filter(|p| !ignored.contains(*p))
            .cloned()
            .collect();
        if paths.len() > 1 {
            out.push(DuplicateGroup { paths });
        }
    }
    out
}

/// Mode-specific data for building [`ViewData`]. Handlers in [`view_data_for_user_selected_mode`] differentiate behavior.
pub enum UserSelectedSource<'a> {
    Duplicates {
        groups: &'a [DuplicateGroup],
    },
    Lenses {
        lens_names: &'a [String],
        db_path: &'a Path,
    },
}

/// Build [`ViewData`] for Duplicates or Lenses from [`UserSelectedSource`]. Single general path; source decides category names and content rows.
pub fn view_data_for_user_selected_mode(
    state: &setup::UblxState,
    source: &UserSelectedSource<'_>,
) -> setup::ViewData {
    let search_query = state.search.query.trim();

    let (filtered_names, dup_groups_in_order): (Vec<String>, Option<Vec<&DuplicateGroup>>) =
        match source {
            UserSelectedSource::Duplicates { groups } => {
                let matching = duplicate_groups_matching_search(groups, search_query);
                let labels = disambiguated_duplicate_labels(&matching);
                (labels, Some(matching))
            }
            UserSelectedSource::Lenses { lens_names, .. } => {
                let names = if search_query.is_empty() {
                    lens_names.to_vec()
                } else if lens_names.len() >= PARALLEL.user_selected_filter {
                    lens_names
                        .par_iter()
                        .filter(|n| fuzzy_matches_field(n, search_query))
                        .cloned()
                        .collect()
                } else {
                    lens_names
                        .iter()
                        .filter(|n| fuzzy_matches_field(n, search_query))
                        .cloned()
                        .collect()
                };
                (names, None)
            }
        };

    let category_list_len = filtered_names.len().max(1);
    let cat_idx = clamp_selection(
        state.panels.category_state.selected().unwrap_or(0),
        category_list_len,
    );

    let path_rows: Vec<setup::TuiRow> = match (&source, dup_groups_in_order.as_ref()) {
        (UserSelectedSource::Duplicates { .. }, Some(matching)) => matching
            .get(cat_idx)
            .map(|g| {
                g.paths
                    .iter()
                    .map(|p| (p.clone(), String::new(), 0u64))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        (UserSelectedSource::Lenses { db_path, .. }, _) => filtered_names
            .get(cat_idx)
            .map(String::as_str)
            .and_then(|name| db_ops::load_lens_paths(db_path, name).ok())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let contents = view_data::filter_contents_by_search(path_rows, search_query);
    view_data::build_user_selected_mode_view_data(
        state.main_mode,
        filtered_names,
        contents,
        state.panels.content_sort,
    )
}

/// `ViewData` for Duplicates mode (delegates to [`view_data_for_user_selected_mode`]).
pub fn view_data_for_duplicates_mode(
    state: &setup::UblxState,
    groups: &[DuplicateGroup],
) -> setup::ViewData {
    let filtered = filter_duplicate_groups_for_view(groups, &state.duplicate_ignored_paths);
    view_data_for_user_selected_mode(state, &UserSelectedSource::Duplicates { groups: &filtered })
}

/// `ViewData` for Lenses mode (delegates to [`view_data_for_user_selected_mode`]).
pub fn view_data_for_lenses_mode(
    state: &setup::UblxState,
    lens_names: &[String],
    db_path: &Path,
) -> setup::ViewData {
    view_data_for_user_selected_mode(
        state,
        &UserSelectedSource::Lenses {
            lens_names,
            db_path,
        },
    )
}
