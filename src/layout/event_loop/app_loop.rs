//! Main app loop: one tick = prune toasts, snapshot handling, build view + right content, draw, input.

use std::io;
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::config::{OPERATION_NAME, UblxOpts};
use crate::engine::db_ops;
use crate::handlers::{applets, core::reapply_terminal_after_editor, snapshot, viewing};
use crate::layout::{setup, themes};
use crate::render::{DrawFrameArgs, draw_ublx_frame};
use crate::ui::input::{MainTabFlags, handle_ublx_input};
use crate::utils::notifications;

use super::delta::{build_delta_view_data, clamp_delta_selection, view_data_for_delta_mode};
use super::params::RunUblxParams;
use super::snapshot::load_snapshot_for_tui;
use super::user_selected::{view_data_for_duplicates_mode, view_data_for_lenses_mode};
use super::view_data::build_view_data;
use super::view_data::clamp_two_pane_selection;

/// Runs until the user quits. Call from [crate::handlers::core::run_ublx] after terminal setup.
///
/// Per-tick: prune toasts → handle snapshot request/done → build view + right content → draw → input (quit breaks).
pub fn main_app_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) -> io::Result<()> {
    loop {
        if run_tick(terminal, state, categories, all_rows, params, ublx_opts)? {
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
    ublx_opts: &mut UblxOpts,
) -> io::Result<bool> {
    applets::settings::on_first_tick(state, params);

    // Drain completed duplicate load from background thread (non-blocking).
    if let Some(rx) = params.duplicate_groups_rx.as_ref()
        && let Ok(groups) = rx.try_recv()
    {
        params.duplicate_groups_rx = None;
        applets::dupe_finder::on_groups_received(state, params, groups);
    }

    if let Some(rx) = params.config_reload_rx.as_ref()
        && rx.try_recv().is_ok()
    {
        applets::settings::on_config_reload(state, params, ublx_opts);
    }

    applets::dupe_finder::spawn_if_requested(state, params);

    prune_toasts(state);
    handle_snapshot_request(state, params);
    handle_snapshot_done(state, categories, all_rows, params);
    poll_snapshot_if_due(state, categories, all_rows, params);

    if params.dev {
        notifications::move_log_events();
    }

    let (view, mut right_content, delta_data, rows_for_draw) =
        build_view_and_right_content(state, categories.as_slice(), all_rows.as_slice(), params);
    if right_content.viewer_can_open {
        right_content.open_hint_label =
            applets::opener::open_hint_label(ublx_opts.editor_path.as_deref()).map(String::from);
    }

    let latest_snapshot_ns = db_ops::load_delta_log_snapshot_timestamps(params.db_path)
        .ok()
        .and_then(|v| v.into_iter().next());
    let theme_name_owned = theme_name_for_tick(state, params);
    let theme_name = theme_name_owned.as_deref();
    let has_duplicates =
        !params.duplicate_groups.is_empty() || params.duplicate_groups_rx.is_some();
    let has_lenses = !params.lens_names.is_empty();
    {
        let draw_inputs = DrawInputs {
            params,
            delta_data: delta_data.as_ref(),
            rows_for_draw,
            theme_name,
            latest_snapshot_ns,
        };
        draw_one_frame(terminal, state, &view, &right_content, &draw_inputs)?;
    }
    let quit = handle_ublx_input(
        state,
        &view,
        &right_content,
        Some((params.dir_to_ublx, theme_name)),
        MainTabFlags {
            has_duplicates,
            has_lenses,
        },
        params,
        ublx_opts,
    )?;
    if state.refresh_terminal_after_editor {
        state.refresh_terminal_after_editor = false;
        reapply_terminal_after_editor()?;
        terminal.clear()?;
        let draw_inputs = DrawInputs {
            params,
            delta_data: delta_data.as_ref(),
            rows_for_draw,
            theme_name,
            latest_snapshot_ns,
        };
        draw_one_frame(terminal, state, &view, &right_content, &draw_inputs)?;
    }
    Ok(quit)
}

/// Inputs for building draw args and drawing one frame. Built once per tick and reused for the normal draw and optional post-editor refresh.
struct DrawInputs<'a> {
    params: &'a RunUblxParams<'a>,
    delta_data: Option<&'a setup::DeltaViewData>,
    rows_for_draw: Option<&'a [setup::TuiRow]>,
    theme_name: Option<&'a str>,
    latest_snapshot_ns: Option<i64>,
}

/// Draw one frame using current view and right content. Used for the normal tick draw and for the post-editor refresh.
fn draw_one_frame<'a>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    draw_inputs: &DrawInputs<'a>,
) -> io::Result<()> {
    let draw_args = build_draw_args(
        draw_inputs.params,
        draw_inputs.delta_data,
        draw_inputs.rows_for_draw,
        draw_inputs.theme_name,
        draw_inputs.latest_snapshot_ns,
    );
    terminal
        .draw(|f| draw_ublx_frame(f, state, view, right_content, &draw_args))
        .map(|_| ())
}

/// Build [DrawFrameArgs] from params and per-tick values.
fn build_draw_args<'a>(
    params: &'a RunUblxParams<'_>,
    delta_data: Option<&'a setup::DeltaViewData>,
    rows_for_draw: Option<&'a [setup::TuiRow]>,
    theme_name: Option<&'a str>,
    latest_snapshot_ns: Option<i64>,
) -> DrawFrameArgs<'a> {
    DrawFrameArgs {
        delta_data,
        all_rows: rows_for_draw,
        dir_to_ublx: Some(params.dir_to_ublx),
        theme_name,
        transparent: params.transparent,
        layout: &params.layout,
        latest_snapshot_ns,
        dev: params.dev,
        duplicate_groups: if params.duplicate_groups.is_empty() {
            None
        } else {
            Some(params.duplicate_groups.as_slice())
        },
        lens_names: if params.lens_names.is_empty() {
            None
        } else {
            Some(params.lens_names.as_slice())
        },
    }
}

fn prune_toasts(state: &mut setup::UblxState) {
    state
        .toasts
        .slots
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

    let (c, r) =
        load_snapshot_for_tui(params.db_path, db_ops::SnapshotReaderPreference::PreferUblx);
    *categories = c;
    *all_rows = r;
    state.snapshot_poll_deadline = None;
    state.snapshot_done_received = true;
    // Duplicates are lazy-loaded when user switches to that tab; don't block here.

    if let Some(b) = params.bumper {
        snapshot::push_snapshot_done_to_bumper(b, added, mod_count, removed);
        let op = OPERATION_NAME.snapshot();
        notifications::show_toast_slot(
            &mut state.toasts.slots,
            b,
            Some(op.as_str()),
            &mut state.toasts.consumed_per_operation,
        );
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
    let (c, r) = load_snapshot_for_tui(params.db_path, db_ops::SnapshotReaderPreference::PreferTmp);
    if !c.is_empty() || !r.is_empty() {
        *categories = c;
        *all_rows = r;
    }
    state.snapshot_poll_deadline = Some(now + SNAPSHOT_POLL_INTERVAL);
}

/// Shared path for Duplicates and Lenses: clamp selection on `view`, then resolve right-pane content. Caller builds `view` first so no closure captures `state` (avoids E0502).
fn build_view_and_right_for_user_selected_mode(
    state: &mut setup::UblxState,
    params: &RunUblxParams<'_>,
    db_path_for_read: &std::path::Path,
    view: setup::ViewData,
) -> (setup::ViewData, setup::RightPaneContent) {
    clamp_two_pane_selection(state, &view);
    let right_content = viewing::resolve_right_pane_content(
        state,
        params.dir_to_ublx,
        db_path_for_read,
        &view,
        None,
    );
    (view, right_content)
}

/// Expands to: build view from `$view`, then [build_view_and_right_for_user_selected_mode]; returns `(view, right_content, None)`.
macro_rules! build_view_and_right_user_selected_mode {
    ($state:expr, $params:expr, $db_path:expr, $view:expr) => {{
        let view = $view;
        let (view, right_content) =
            build_view_and_right_for_user_selected_mode($state, $params, $db_path, view);
        (view, right_content, None)
    }};
}

/// Build view data and right-pane content for the current mode (Snapshot, Delta, Duplicates, Lenses). Returns (view, right_content, delta_data for draw, rows slice for draw).
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
        let d = build_delta_view_data(params.db_path);
        let view = view_data_for_delta_mode(state, &d);
        clamp_delta_selection(state, &view);
        let right_content = setup::RightPaneContent::empty();
        (view, right_content, Some(d), None)
    } else {
        let db_path_for_read =
            db_ops::snapshot_read_path_for_tui(params.db_path, !state.snapshot_done_received);
        let (view, right_content, rows_for_draw) = if state.main_mode == setup::MainMode::Duplicates
        {
            build_view_and_right_user_selected_mode!(
                state,
                params,
                &db_path_for_read,
                view_data_for_duplicates_mode(state, &params.duplicate_groups)
            )
        } else if state.main_mode == setup::MainMode::Lenses {
            build_view_and_right_user_selected_mode!(
                state,
                params,
                &db_path_for_read,
                view_data_for_lenses_mode(state, &params.lens_names, &db_path_for_read)
            )
        } else {
            let view = build_view_data(state, categories, all_rows);
            let right_content = viewing::resolve_right_pane_content(
                state,
                params.dir_to_ublx,
                &db_path_for_read,
                &view,
                Some(all_rows),
            );
            (view, right_content, Some(all_rows))
        };
        (view, right_content, None, rows_for_draw)
    }
}

/// Return owned theme name so callers don't hold a borrow of state (avoids borrow conflicts with draw/input).
fn theme_name_for_tick(state: &setup::UblxState, params: &RunUblxParams<'_>) -> Option<String> {
    if state.theme.selector_visible {
        Some(
            themes::theme_options()[state.theme.selector_index]
                .display_name
                .to_string(),
        )
    } else {
        state
            .theme
            .override_name
            .clone()
            .or_else(|| params.theme.clone())
    }
}
