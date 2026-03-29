//! Off-thread right-pane resolve for file rows: DB + disk read + zahir JSON, with generation IDs so stale results are dropped.

use std::path::Path;

use tokio::sync::mpsc::UnboundedSender;

use super::core::{self, NonDirectoryRightPaneBuild};

use crate::app::tokio_rt;
use crate::layout::setup::{
    CATEGORY_DIRECTORY, RightPaneAsyncReady, RightPaneContent, TuiRow, UblxState, ViewData,
};
use crate::utils;

struct RightPaneAsyncSpawnJob {
    generation: u64,
    path: String,
    category: String,
    size: u64,
    full_path: std::path::PathBuf,
    enable_enhance_all: bool,
    db_path: std::path::PathBuf,
    tx: UnboundedSender<RightPaneAsyncReady>,
}

fn spawn_file_resolve(job: RightPaneAsyncSpawnJob) {
    let RightPaneAsyncSpawnJob {
        generation,
        path,
        category,
        size,
        full_path,
        enable_enhance_all,
        db_path,
        tx,
    } = job;

    tokio_rt::runtime().spawn(async move {
        let path_for_job = path.clone();
        let res = tokio::task::spawn_blocking(move || {
            let viewer_mtime_ns =
                crate::engine::db_ops::load_mtime_for_path(&db_path, &path_for_job)
                    .ok()
                    .flatten();
            core::build_non_directory_right_pane_inner(&NonDirectoryRightPaneBuild {
                db_path: &db_path,
                path: &path_for_job,
                category: &category,
                size,
                full_path: &full_path,
                viewer_mtime_ns,
                enable_enhance_all,
                disk_cache_hint: None,
            })
        })
        .await;

        let (content, disk_cache) = res.unwrap_or_else(|_| (RightPaneContent::empty(), None));
        let _ = tx.send(RightPaneAsyncReady {
            generation,
            path,
            content,
            disk_cache,
        });
    });
}

fn poll_matching_completions(state: &mut UblxState, selected_path: &str) {
    let Some(rx) = state.right_pane_async.rx.as_mut() else {
        return;
    };
    while let Ok(msg) = rx.try_recv() {
        if msg.generation != state.right_pane_async.generation {
            continue;
        }
        if msg.path != selected_path {
            continue;
        }
        state.viewer_disk_cache = msg.disk_cache;
        state.right_pane_async.displayed = msg.content;
    }
}

fn drain_async_channel(state: &mut UblxState) {
    let Some(rx) = state.right_pane_async.rx.as_mut() else {
        return;
    };
    while rx.try_recv().is_ok() {}
}

/// Snapshot / Duplicates / Lenses: tree + zahir resolve. File rows load off-thread; directory rows stay sync.
///
/// While a new file row is loading, the last fully resolved [`RightPaneContent`] is shown (no empty flash).
pub fn drive_right_pane_async(
    state: &mut UblxState,
    tx: Option<&UnboundedSender<RightPaneAsyncReady>>,
    dir_to_ublx: &Path,
    db_path: &Path,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    enable_enhance_all: bool,
) -> RightPaneContent {
    let Some(tx) = tx else {
        return core::resolve_right_pane_content(
            state,
            dir_to_ublx,
            db_path,
            view,
            all_rows,
            enable_enhance_all,
        );
    };

    let selected = state
        .panels
        .content_state
        .selected()
        .and_then(|i| view.row_at(i, all_rows));

    let Some(row) = selected else {
        state.right_pane_async.generation = state.right_pane_async.generation.saturating_add(1);
        state.right_pane_async.last_spawn_path = String::new();
        state.viewer_disk_cache = None;
        state.cached_tree = None;
        drain_async_channel(state);
        state.right_pane_async.displayed = RightPaneContent::empty();
        return RightPaneContent::empty();
    };

    let (path, category, size) = (&row.0, row.1.as_str(), row.2);
    let path_str = path.as_str();

    poll_matching_completions(state, path_str);

    let full_path = utils::resolve_under_root(dir_to_ublx, path_str);

    if category == CATEGORY_DIRECTORY || full_path.is_dir() {
        state.right_pane_async.last_spawn_path = String::new();
        let content = core::resolve_right_pane_content(
            state,
            dir_to_ublx,
            db_path,
            view,
            all_rows,
            enable_enhance_all,
        );
        state.right_pane_async.displayed = content.clone();
        return content;
    }

    // Spawn only when the selection path changed. While a job is in flight, `displayed` may still
    // show the previous row; do not re-spawn every frame.
    let need_spawn = state.right_pane_async.last_spawn_path != path_str;

    if need_spawn {
        state.right_pane_async.generation = state.right_pane_async.generation.saturating_add(1);
        let generation = state.right_pane_async.generation;
        state.right_pane_async.last_spawn_path = path_str.to_string();

        spawn_file_resolve(RightPaneAsyncSpawnJob {
            generation,
            path: path_str.to_string(),
            category: category.to_string(),
            size,
            full_path,
            enable_enhance_all,
            db_path: db_path.to_path_buf(),
            tx: tx.clone(),
        });
    }

    state.right_pane_async.displayed.clone()
}
