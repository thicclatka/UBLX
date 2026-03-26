//! One UI tick: side effects → view build → draw → input.

use std::io;
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;

use super::frame::{DrawInputs, draw_one_frame, theme_name_for_tick};
use super::view_build::build_view_and_right_content;

use crate::app::{RunUblxParams, load_snapshot_for_tui};
use crate::config::UblxOpts;
use crate::engine::{db_ops, orchestrator};
use crate::handlers::{applets, reapply_terminal_after_editor, snapshot};
use crate::layout::setup;
use crate::ui::{
    MainTabFlags,
    input::{InputContext, handle_ublx_input},
    snapshot::{show_force_full_enhance_started_toast, show_snapshot_completed_toast},
};
use crate::utils::notifications;

const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// One tick: update toasts/snapshot, build view and right content, draw, handle input. Returns true if quit requested.
pub fn run_tick(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) -> io::Result<bool> {
    tick_applets_and_io(state, categories, all_rows, params, ublx_opts);
    tick_toasts_and_snapshot(state, categories, all_rows, params, ublx_opts);

    if params.display.dev {
        notifications::move_log_events();
    }

    let (view, right_content, delta_data, rows_for_draw) = build_view_and_right_content(
        state,
        categories.as_slice(),
        all_rows.as_slice(),
        params,
        ublx_opts.enable_enhance_all,
    );

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
    let layout_for_input = params.layout.clone();
    let quit = handle_ublx_input(
        state,
        InputContext {
            view: &view,
            right_content: &right_content,
            theme_ctx: Some((params.dir_to_ublx, theme_name)),
            frame_area: {
                let sz = terminal.size()?;
                Rect::new(0, 0, sz.width, sz.height)
            },
            layout: &layout_for_input,
            tabs: MainTabFlags {
                has_duplicates,
                has_lenses,
            },
        },
        params,
        ublx_opts,
    )?;
    if state.session.tick.refresh_terminal_after_editor {
        state.session.tick.refresh_terminal_after_editor = false;
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

/// First-tick hooks, snapshot row reload, background channel drains, dupe-finder spawn.
fn tick_applets_and_io(
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) {
    applets::settings::on_first_tick(state, params);

    if params.startup.pending_force_full_enhance_toast {
        params.startup.pending_force_full_enhance_toast = false;
        if !state.session.reload.force_full_enhance_toast_shown {
            state.session.reload.force_full_enhance_toast_shown = true;
            show_force_full_enhance_started_toast(state, params);
        }
    }

    if state.session.reload.snapshot_rows {
        let (c, r) =
            load_snapshot_for_tui(params.db_path, db_ops::SnapshotReaderPreference::PreferUblx);
        *categories = c;
        *all_rows = r;
        state.session.reload.snapshot_rows = false;
    }

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
}

fn tick_toasts_and_snapshot(
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
) {
    prune_toasts(state);
    handle_snapshot_request(state, params, ublx_opts);
    handle_snapshot_done(state, categories, all_rows, params);
    poll_snapshot_if_due(state, categories, all_rows, params);
}

fn prune_toasts(state: &mut setup::UblxState) {
    state
        .toasts
        .slots
        .retain(|s| Instant::now() < s.visible_until);
}

fn handle_snapshot_request(
    state: &mut setup::UblxState,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
) {
    if !state.snapshot_bg.requested {
        return;
    }
    if orchestrator::should_force_full_zahir(ublx_opts)
        && !state.session.reload.force_full_enhance_toast_shown
    {
        params.startup.pending_force_full_enhance_toast = true;
    }
    snapshot::spawn_snapshot_from_dir_db(
        params.dir_to_ublx,
        params.db_path,
        params.snapshot_done_tx.as_ref(),
        params.bumper,
        Some(ublx_opts),
    );
    state.snapshot_bg.requested = false;
    state.snapshot_bg.done_received = false; // start polling .ublx_tmp for live progress
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
    state.snapshot_bg.poll_deadline = None;
    state.snapshot_bg.done_received = true;
    if state.snapshot_bg.defer_snapshot_after_current {
        state.snapshot_bg.defer_snapshot_after_current = false;
        state.snapshot_bg.requested = true;
    }
    // Duplicates are lazy-loaded when user switches to that tab; don't block here.

    show_snapshot_completed_toast(state, params, added, mod_count, removed);
}

/// When a background snapshot is running, periodically reload from the DB (e.g. .`ublx_tmp`) so the TUI shows live progress.
fn poll_snapshot_if_due(
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &RunUblxParams<'_>,
) {
    let Some(ref _rx) = params.snapshot_done_rx else {
        return;
    };
    if state.snapshot_bg.done_received {
        return;
    }
    let now = Instant::now();
    let due = state.snapshot_bg.poll_deadline.is_none_or(|d| now >= d);
    if !due {
        return;
    }
    let (c, r) = load_snapshot_for_tui(params.db_path, db_ops::SnapshotReaderPreference::PreferTmp);
    if !c.is_empty() || !r.is_empty() {
        *categories = c;
        *all_rows = r;
    }
    state.snapshot_bg.poll_deadline = Some(now + SNAPSHOT_POLL_INTERVAL);
}
