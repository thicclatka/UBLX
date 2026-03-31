//! Build [`setup::ViewData`] and [`setup::RightPaneContent`] for the current main mode.

use crate::app::{
    RunUblxParams,
    delta::{build_delta_view_data, clamp_delta_selection, view_data_for_delta_mode},
    user_selected::{view_data_for_duplicates_mode, view_data_for_lenses_mode},
    view_data::{build_view_data, clamp_two_pane_selection},
};
use crate::engine::db_ops;
use crate::handlers::viewing::async_ops;
use crate::layout::setup;
use crate::utils::PerfGuard;

fn apply_sort_anchor_selection(
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
) {
    let Some(anchor) = state.panels.sort_anchor_path.take() else {
        return;
    };
    let idx = view
        .iter_contents(all_rows)
        .position(|(path, _, _)| path == &anchor);
    if let Some(i) = idx {
        state.panels.content_state.select(Some(i));
    }
}

/// Shared path for Duplicates and Lenses: clamp selection on `view`, then resolve right-pane content. Caller builds `view` first so no closure captures `state` (avoids E0502).
fn build_view_and_right_for_user_selected_mode(
    state: &mut setup::UblxState,
    params: &RunUblxParams<'_>,
    db_path_for_read: &std::path::Path,
    view: setup::ViewData,
    enable_enhance_all: bool,
) -> (setup::ViewData, setup::RightPaneContent) {
    clamp_two_pane_selection(state, &view);
    apply_sort_anchor_selection(state, &view, None);
    let right_content = async_ops::drive_right_pane_async(
        state,
        params.right_pane_async_tx.as_ref(),
        &params.dir_to_ublx,
        db_path_for_read,
        &view,
        None,
        enable_enhance_all,
    );
    (view, right_content)
}

/// Expands to: build view from `$view`, then [`build_view_and_right_for_user_selected_mode`]; returns `(view, right_content, None)`.
macro_rules! build_view_and_right_user_selected_mode {
    ($state:expr, $params:expr, $db_path:expr, $view:expr, $enable_enhance:expr) => {{
        let view = $view;
        let (view, right_content) = build_view_and_right_for_user_selected_mode(
            $state,
            $params,
            $db_path,
            view,
            $enable_enhance,
        );
        (view, right_content, None)
    }};
}

/// Build view data and right-pane content for the current mode (Snapshot, Delta, Duplicates, Lenses). Returns (view, `right_content`, `delta_data` for draw, rows slice for draw).
pub fn build_view_and_right_content<'a>(
    state: &mut setup::UblxState,
    categories: &[String],
    all_rows: &'a [setup::TuiRow],
    params: &RunUblxParams<'_>,
    enable_enhance_all: bool,
) -> (
    setup::ViewData,
    setup::RightPaneContent,
    Option<setup::DeltaViewData>,
    Option<&'a [setup::TuiRow]>,
) {
    let _perf = PerfGuard::new("view_build.build_view_and_right_content");
    // If Duplicates/Lenses has no data, switch to Snapshot to avoid empty or hanging loading screen.
    if state.main_mode == setup::MainMode::Duplicates
        && params.duplicate_groups.is_empty()
        && params.duplicate_groups_rx.is_none()
    {
        state.main_mode = setup::MainMode::Snapshot;
    }
    if state.main_mode == setup::MainMode::Lenses && params.lens_names.is_empty() {
        state.main_mode = setup::MainMode::Snapshot;
    }

    if state.main_mode == setup::MainMode::Delta {
        let d = build_delta_view_data(&params.db_path);
        let view = view_data_for_delta_mode(state, &d);
        clamp_delta_selection(state, &view);
        apply_sort_anchor_selection(state, &view, None);
        let right_content = setup::RightPaneContent::empty();
        (view, right_content, Some(d), None)
    } else if state.main_mode == setup::MainMode::Settings {
        let view = setup::ViewData {
            filtered_categories: vec![],
            contents: setup::ViewContents::SnapshotIndices(vec![]),
            category_list_len: 1,
            content_len: 0,
        };
        (view, setup::RightPaneContent::empty(), None, None)
    } else {
        let db_path_for_read =
            db_ops::snapshot_read_path_for_tui(&params.db_path, !state.snapshot_bg.done_received);
        let snapshot_mtimes = if state.main_mode == setup::MainMode::Snapshot
            && state.panels.content_sort.snapshot_key == setup::SnapshotSortKey::Mod
        {
            db_ops::load_snapshot_path_mtimes(&db_path_for_read).ok()
        } else {
            None
        };
        let (view, right_content, rows_for_draw) = if state.main_mode == setup::MainMode::Duplicates
        {
            build_view_and_right_user_selected_mode!(
                state,
                params,
                &db_path_for_read,
                view_data_for_duplicates_mode(state, &params.duplicate_groups),
                enable_enhance_all
            )
        } else if state.main_mode == setup::MainMode::Lenses {
            build_view_and_right_user_selected_mode!(
                state,
                params,
                &db_path_for_read,
                view_data_for_lenses_mode(state, &params.lens_names, &db_path_for_read),
                enable_enhance_all
            )
        } else {
            let view = build_view_data(state, categories, all_rows, snapshot_mtimes.as_ref());
            apply_sort_anchor_selection(state, &view, Some(all_rows));
            let right_content = async_ops::drive_right_pane_async(
                state,
                params.right_pane_async_tx.as_ref(),
                &params.dir_to_ublx,
                &db_path_for_read,
                &view,
                Some(all_rows),
                enable_enhance_all,
            );
            (view, right_content, Some(all_rows))
        };
        (view, right_content, None, rows_for_draw)
    }
}
