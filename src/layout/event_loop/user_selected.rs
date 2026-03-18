//! User-selected list modes: Duplicates and Lenses. Same two-pane structure (categories + contents);
//! only the source of category names and content rows differs.

use std::path::Path;

use crate::engine::db_ops::{self, DuplicateGroup};
use crate::layout::setup;
use crate::utils::format::clamp_selection;

use super::view_data;

/// Mode-specific data for building [ViewData]. Handlers in [view_data_for_user_selected_mode] differentiate behavior.
pub enum UserSelectedSource<'a> {
    Duplicates {
        groups: &'a [DuplicateGroup],
    },
    Lenses {
        lens_names: &'a [String],
        db_path: &'a Path,
    },
}

/// Build [ViewData] for Duplicates or Lenses from [UserSelectedSource]. Single general path; source decides category names and content rows.
pub fn view_data_for_user_selected_mode(
    state: &setup::UblxState,
    source: UserSelectedSource<'_>,
) -> setup::ViewData {
    let search_query = state.search.query.trim();

    let filtered_names: Vec<String> = match &source {
        UserSelectedSource::Duplicates { groups } => {
            if search_query.is_empty() {
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
            }
        }
        UserSelectedSource::Lenses { lens_names, .. } => {
            if search_query.is_empty() {
                lens_names.to_vec()
            } else {
                lens_names
                    .iter()
                    .filter(|n| n.contains(search_query))
                    .cloned()
                    .collect()
            }
        }
    };

    let category_list_len = filtered_names.len().max(1);
    let cat_idx = clamp_selection(
        state.panels.category_state.selected().unwrap_or(0),
        category_list_len,
    );

    let path_rows: Vec<setup::TuiRow> = match &source {
        UserSelectedSource::Duplicates { groups } => filtered_names
            .get(cat_idx)
            .and_then(|name| groups.iter().find(|g| g.representative_name() == name))
            .map(|g| {
                g.paths
                    .iter()
                    .map(|p| (p.clone(), String::new(), 0u64))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        UserSelectedSource::Lenses { db_path, .. } => filtered_names
            .get(cat_idx)
            .map(String::as_str)
            .and_then(|name| db_ops::load_lens_paths(db_path, name).ok())
            .unwrap_or_default(),
    };

    let contents = view_data::filter_contents_by_search(path_rows, search_query);
    view_data::build_user_selected_mode_view_data(filtered_names, contents)
}

/// ViewData for Duplicates mode (delegates to [view_data_for_user_selected_mode]).
pub fn view_data_for_duplicates_mode(
    state: &setup::UblxState,
    groups: &[DuplicateGroup],
) -> setup::ViewData {
    view_data_for_user_selected_mode(state, UserSelectedSource::Duplicates { groups })
}

/// ViewData for Lenses mode (delegates to [view_data_for_user_selected_mode]).
pub fn view_data_for_lenses_mode(
    state: &setup::UblxState,
    lens_names: &[String],
    db_path: &Path,
) -> setup::ViewData {
    view_data_for_user_selected_mode(
        state,
        UserSelectedSource::Lenses {
            lens_names,
            db_path,
        },
    )
}
