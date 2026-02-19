use std::io;
use std::path::Path;
use std::sync::mpsc;
use std::time::Instant;

use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::config::TOAST_CONFIG;
use crate::engine::db_ops;
use crate::handlers::snapshot;
use crate::layout::{filter, setup, viewing_pane};
use crate::render::{draw_ublx_frame, DrawFrameArgs};
use crate::ui::input::handle_ublx_input;
use crate::utils::{format::format_timestamp_ns, notifications};

/// Parameters for the TUI event loop.
pub struct RunUblxParams<'a> {
    pub db_path: &'a Path,
    pub dir_to_ublx: &'a Path,
    pub snapshot_done_rx: Option<mpsc::Receiver<(usize, usize, usize)>>,
    pub snapshot_done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    pub bumper: Option<&'a notifications::BumperBuffer>,
    pub dev: bool,
    pub theme: Option<String>,
    pub transparent: bool,
}

/// Sort categories and contents alphanumeric. Rows are ordered by path only.
fn sort_categories_and_rows(categories: &mut [String], all_rows: &mut [setup::TuiRow]) {
    categories.sort();
    all_rows.sort_by(|a, b| a.0.cmp(&b.0));
}

/// Load snapshot categories and rows for the TUI, sorted. Use at startup and when a snapshot run completes.
fn load_snapshot_for_tui(db_path: &Path) -> (Vec<String>, Vec<setup::TuiRow>) {
    let mut categories = db_ops::load_snapshot_categories(db_path).unwrap_or_default();
    let mut all_rows = db_ops::load_snapshot_rows_for_tui(db_path, None).unwrap_or_default();
    sort_categories_and_rows(&mut categories, &mut all_rows);
    (categories, all_rows)
}

/// Compute filtered categories and contents from search + category selection; clamp list
/// selection and reset preview scroll when selection changes.
pub fn build_view_data(
    state: &mut setup::UblxState,
    categories: &[String],
    all_rows: &[setup::TuiRow],
) -> setup::ViewData {
    let search_query = state.search_query.trim();
    let filtered_categories =
        filter::categories_for_search(categories, all_rows, search_query);
    let category_idx = state.category_state.selected().unwrap_or(0);
    let selected_category = if category_idx == 0 {
        None
    } else {
        filtered_categories
            .get(category_idx - 1)
            .map(String::as_str)
    };
    let contents_indices = filter::content_indices_for_view(
        all_rows,
        selected_category,
        search_query,
    );
    let category_list_len = 1 + filtered_categories.len();
    if category_list_len > 0 {
        let idx = category_idx.min(category_list_len.saturating_sub(1));
        state.category_state.select(Some(idx));
    }
    let content_len = contents_indices.len();
    if content_len > 0 {
        let sel = state
            .content_state
            .selected()
            .unwrap_or(0)
            .min(content_len.saturating_sub(1));
        state.content_state.select(Some(sel));
    } else {
        state.content_state.select(None);
    }
    let content_sel = state.content_state.selected();
    let preview_key = (category_idx, content_sel);
    if state.prev_preview_key.as_ref() != Some(&preview_key) {
        state.preview_scroll = 0;
        state.prev_preview_key = Some(preview_key);
    }
    setup::ViewData {
        filtered_categories,
        contents: setup::ViewContents::SnapshotIndices(contents_indices),
        category_list_len,
        content_len,
    }
}

/// Load delta_log data for Delta mode: overview text (snapshot count + timestamps) and paths per type.
pub fn build_delta_view_data(db_path: &Path) -> setup::DeltaViewData {
    let timestamps = db_ops::load_delta_log_snapshot_timestamps(db_path).unwrap_or_default();
    let snapshot_count = timestamps.len();
    let overview_lines: Vec<String> = std::iter::once(String::new())
        .chain(std::iter::once(format!(
            "{} snapshot(s) (sorted by time; newest first):",
            snapshot_count
        )))
        .chain(std::iter::once(String::new()))
        .chain(
            timestamps
                .into_iter()
                .map(|ns| format!("  • {}", format_timestamp_ns(ns))),
        )
        .collect();
    let overview_text = overview_lines.join("\n");

    let added_paths = build_delta_display_lines(
        db_ops::load_delta_log_rows_by_type(db_path, "added").unwrap_or_default(),
    );
    let mod_paths = build_delta_display_lines(
        db_ops::load_delta_log_rows_by_type(db_path, "mod").unwrap_or_default(),
    );
    let removed_paths = build_delta_display_lines(
        db_ops::load_delta_log_rows_by_type(db_path, "removed").unwrap_or_default(),
    );

    setup::DeltaViewData {
        overview_text,
        added_paths,
        mod_paths,
        removed_paths,
    }
}

/// Group (created_ns, path) by created_ns (newest first) and produce display lines: timestamp then "  path" per path.
fn build_delta_display_lines(rows: Vec<(i64, String)>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_ns: Option<i64> = None;
    for (ns, path) in rows {
        if current_ns != Some(ns) {
            current_ns = Some(ns);
            lines.push(format_timestamp_ns(ns));
        }
        lines.push(format!("  {}", path));
    }
    lines
}

/// Clamp list selection for Delta mode (3 categories, current type's path count).
pub fn clamp_delta_selection(state: &mut setup::UblxState, delta: &setup::DeltaViewData) {
    let cat_idx = state.category_state.selected().unwrap_or(0).min(2);
    state.category_state.select(Some(cat_idx));
    let paths = delta.paths_by_index(cat_idx);
    let len = paths.len();
    if len > 0 {
        let sel = state
            .content_state
            .selected()
            .unwrap_or(0)
            .min(len.saturating_sub(1));
        state.content_state.select(Some(sel));
    } else {
        state.content_state.select(None);
    }
}

/// ViewData for input/navigation when in Delta mode (3 categories, content_len = current type's path count).
/// Uses owned rows (delta list is typically small).
pub fn view_data_for_delta_mode(
    state: &setup::UblxState,
    delta: &setup::DeltaViewData,
) -> setup::ViewData {
    let cat_idx = state.category_state.selected().unwrap_or(0).min(2);
    let paths = delta.paths_by_index(cat_idx);
    let rows: Vec<setup::TuiRow> = paths
        .iter()
        .map(|p| (p.clone(), String::new(), 0u64))
        .collect();
    setup::ViewData {
        filtered_categories: vec!["Added".into(), "Mod".into(), "Removed".into()],
        contents: setup::ViewContents::DeltaRows(rows),
        category_list_len: 3,
        content_len: paths.len(),
    }
}

/// **Classification of each call per tick:**
/// 1. **View data** — `compute_loop_view`: filter categories/contents by search and category, clamp selection, update preview key.
/// 2. **Right-pane content** — `resolve_right_pane_content`: templates/metadata/writing/viewer for current selection.
/// 3. **Draw** — `draw_ublx_frame`: layout and render categories, contents, right pane, search, help.
/// 4. **Input** — `handle_ublx_input`: poll key, map to action, apply to state; returns true if quit.
pub fn run_ublx(params: RunUblxParams<'_>) -> io::Result<()> {
    let (mut categories, mut all_rows) = load_snapshot_for_tui(params.db_path);

    let mut state = setup::UblxState::new();

    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    loop {
        if state
            .toast_visible_until
            .is_some_and(|until| Instant::now() >= until)
        {
            state.toast_visible_until = None;
        }
        if state.snapshot_requested {
            snapshot::spawn_snapshot_from_dir_db(
                params.dir_to_ublx,
                params.db_path,
                params.snapshot_done_tx.as_ref(),
                params.bumper,
            );
            state.snapshot_requested = false;
        }
        if let Some(ref rx) = params.snapshot_done_rx
            && let Ok((added, mod_count, removed)) = rx.try_recv()
        {
            (categories, all_rows) = load_snapshot_for_tui(params.db_path);
            if let Some(b) = params.bumper {
                snapshot::push_snapshot_done_to_bumper(b, added, mod_count, removed);
            }
            state.toast_visible_until = Some(Instant::now() + TOAST_CONFIG.duration);
        }
        if params.dev {
            notifications::move_log_events();
        }

        let (view, right_content, delta_data, rows_for_draw) =
            if state.main_mode == setup::MainMode::Delta {
                let d = build_delta_view_data(params.db_path);
                clamp_delta_selection(&mut state, &d);
                let view = view_data_for_delta_mode(&state, &d);
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
            } else {
                let view = build_view_data(&mut state, &categories, &all_rows);
                let right_content = viewing_pane::resolve_right_pane_content(
                    &mut state,
                    params.dir_to_ublx,
                    params.db_path,
                    &view,
                    Some(&all_rows),
                );
                (view, right_content, None, Some(all_rows.as_slice()))
            };
        let latest_snapshot_ns = (state.main_mode == setup::MainMode::Snapshot)
            .then(|| db_ops::load_delta_log_snapshot_timestamps(params.db_path).ok())
            .flatten()
            .and_then(|v| v.into_iter().next());
        let draw_args = DrawFrameArgs {
            delta_data: delta_data.as_ref(),
            all_rows: rows_for_draw,
            dir_to_ublx: (state.main_mode == setup::MainMode::Snapshot).then_some(params.dir_to_ublx),
            theme_name: params.theme.as_deref(),
            transparent: params.transparent,
            latest_snapshot_ns,
            bumper: params.bumper,
            dev: params.dev,
        };
        terminal.draw(|f| {
            draw_ublx_frame(f, &mut state, &view, &right_content, &draw_args)
        })?;
        if handle_ublx_input(&mut state, &view, &right_content)? {
            break;
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
