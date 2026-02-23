//! Main app loop: one tick = prune toasts, snapshot handling, build view + right content, draw, input.

use std::io;
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::config::OPERATION_NAME;
use crate::engine::db_ops;
use crate::handlers::{snapshot, viewing};
use crate::layout::{setup, themes};
use crate::render::{DrawFrameArgs, draw_ublx_frame};
use crate::ui::input::handle_ublx_input;
use crate::utils::notifications;

use super::delta::{build_delta_view_data, clamp_delta_selection, view_data_for_delta_mode};
use super::duplicates::{clamp_duplicates_selection, view_data_for_duplicates_mode};
use super::params::RunUblxParams;
use super::snapshot::load_snapshot_for_tui;
use super::view_data::build_view_data;
use crate::engine::db_ops::SnapshotReaderPreference;

/// Runs until the user quits. Call from [crate::handlers::core::run_ublx] after terminal setup.
///
/// Per-tick: prune toasts → handle snapshot request/done → build view + right content → draw → input (quit breaks).
pub fn main_app_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
) -> io::Result<()> {
    loop {
        if run_tick(terminal, state, categories, all_rows, params)? {
            break;
        }
    }
    Ok(())
}

/// One tick: update toasts/snapshot, build view and right content, draw, handle input. Returns true if quit requested.
const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_millis(500);

fn run_tick(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
) -> io::Result<bool> {
    // Drain completed duplicate load from background thread (non-blocking).
    if let Some(rx) = params.duplicate_groups_rx.as_ref()
        && let Ok(groups) = rx.try_recv()
    {
        params.duplicate_groups = groups;
        params.duplicate_groups_rx = None;
    }

    if state.duplicate_load_requested
        && params.duplicate_groups.is_empty()
        && params.duplicate_groups_rx.is_none()
    {
        spawn_duplicate_load(params);
        state.duplicate_load_requested = false;
    }

    prune_toasts(state);
    handle_snapshot_request(state, params);
    handle_snapshot_done(state, categories, all_rows, params);
    poll_snapshot_if_due(state, categories, all_rows, params);

    if params.dev {
        notifications::move_log_events();
    }

    let (view, right_content, delta_data, rows_for_draw) =
        build_view_and_right_content(state, categories.as_slice(), all_rows.as_slice(), params);

    let latest_snapshot_ns = db_ops::load_delta_log_snapshot_timestamps(params.db_path)
        .ok()
        .and_then(|v| v.into_iter().next());
    let theme_name_owned = theme_name_for_tick(state, params);
    let theme_name = theme_name_owned.as_deref();
    let draw_args = DrawFrameArgs {
        delta_data: delta_data.as_ref(),
        all_rows: rows_for_draw,
        dir_to_ublx: Some(params.dir_to_ublx),
        theme_name,
        transparent: params.transparent,
        latest_snapshot_ns,
        dev: params.dev,
        duplicate_groups: if params.duplicate_groups.is_empty() {
            None
        } else {
            Some(params.duplicate_groups.as_slice())
        },
        duplicate_groups_loading: params.duplicate_groups_rx.is_some(),
    };

    terminal.draw(|f| draw_ublx_frame(f, state, &view, &right_content, &draw_args))?;
    let has_duplicates =
        !params.duplicate_groups.is_empty() || params.duplicate_groups_rx.is_some();
    handle_ublx_input(
        state,
        &view,
        &right_content,
        Some((params.dir_to_ublx, theme_name)),
        params.bumper,
        params.dev,
        has_duplicates,
    )
}

fn spawn_duplicate_load(params: &mut RunUblxParams<'_>) {
    let db_path = params.db_path.to_path_buf();
    let dir_to_ublx = params.dir_to_ublx.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    params.duplicate_groups_rx = Some(rx);
    std::thread::spawn(move || {
        let groups = db_ops::load_duplicate_groups(&db_path, &dir_to_ublx).unwrap_or_default();
        let _ = tx.send(groups);
    });
}

fn prune_toasts(state: &mut setup::UblxState) {
    state
        .toast_slots
        .retain(|s| Instant::now() < s.visible_until);
}

fn handle_snapshot_request(state: &mut setup::UblxState, params: &RunUblxParams<'_>) {
    if !state.snapshot_requested {
        return;
    }
    snapshot::spawn_snapshot_from_dir_db(
        params.dir_to_ublx,
        params.db_path,
        params.snapshot_done_tx.as_ref(),
        params.bumper,
    );
    state.snapshot_requested = false;
    state.snapshot_done_received = false; // start polling .ublx_tmp for live progress
}

fn handle_snapshot_done(
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
) {
    let Some(ref rx) = params.snapshot_done_rx else {
        return;
    };
    let Ok((added, mod_count, removed)) = rx.try_recv() else {
        return;
    };

    let (c, r) = load_snapshot_for_tui(params.db_path, SnapshotReaderPreference::PreferUblx);
    *categories = c;
    *all_rows = r;
    state.snapshot_poll_deadline = None;
    state.snapshot_done_received = true;
    // Duplicates are lazy-loaded when user switches to that tab; don't block here.

    if let Some(b) = params.bumper {
        snapshot::push_snapshot_done_to_bumper(b, added, mod_count, removed);
        let op = OPERATION_NAME.snapshot();
        notifications::show_toast_slot(&mut state.toast_slots, b, Some(op.as_str()), params.dev);
    }
}

/// When a background snapshot is running, periodically reload from the DB (e.g. .ublx_tmp) so the TUI shows live progress.
fn poll_snapshot_if_due(
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &RunUblxParams<'_>,
) {
    let Some(ref _rx) = params.snapshot_done_rx else {
        return;
    };
    if state.snapshot_done_received {
        return;
    }
    let now = Instant::now();
    let due = state.snapshot_poll_deadline.is_none_or(|d| now >= d);
    if !due {
        return;
    }
    let (c, r) = load_snapshot_for_tui(params.db_path, SnapshotReaderPreference::PreferTmp);
    if !c.is_empty() || !r.is_empty() {
        *categories = c;
        *all_rows = r;
    }
    state.snapshot_poll_deadline = Some(now + SNAPSHOT_POLL_INTERVAL);
}

/// Build view data and right-pane content for the current mode (Snapshot, Delta, or Duplicates). Returns (view, right_content, delta_data for draw, rows slice for draw).
fn build_view_and_right_content<'a>(
    state: &mut setup::UblxState,
    categories: &[String],
    all_rows: &'a [setup::TuiRow],
    params: &RunUblxParams<'_>,
) -> (
    setup::ViewData,
    setup::RightPaneContent,
    Option<setup::DeltaViewData>,
    Option<&'a [setup::TuiRow]>,
) {
    if state.main_mode == setup::MainMode::Delta {
        let d = build_delta_view_data(params.db_path);
        let view = view_data_for_delta_mode(state, &d);
        clamp_delta_selection(state, &view);
        let right_content = setup::RightPaneContent {
            templates: String::new(),
            metadata: None,
            writing: None,
            viewer: None,
            viewer_path: None,
            viewer_byte_size: None,
            viewer_mtime_ns: None,
        };
        (view, right_content, Some(d), None)
    } else if state.main_mode == setup::MainMode::Duplicates {
        let view = view_data_for_duplicates_mode(state, &params.duplicate_groups);
        clamp_duplicates_selection(state, &view);
        let right_content = viewing::resolve_right_pane_content(
            state,
            params.dir_to_ublx,
            params.db_path,
            &view,
            None,
        );
        (view, right_content, None, None)
    } else {
        let view = build_view_data(state, categories, all_rows);
        let right_content = viewing::resolve_right_pane_content(
            state,
            params.dir_to_ublx,
            params.db_path,
            &view,
            Some(all_rows),
        );
        (view, right_content, None, Some(all_rows))
    }
}

/// Return owned theme name so callers don't hold a borrow of state (avoids borrow conflicts with draw/input).
fn theme_name_for_tick(state: &setup::UblxState, params: &RunUblxParams<'_>) -> Option<String> {
    if state.theme_selector_visible {
        Some(
            themes::theme_options()[state.theme_selector_index]
                .display_name
                .to_string(),
        )
    } else {
        state
            .theme_override
            .clone()
            .or_else(|| params.theme.clone())
    }
}
