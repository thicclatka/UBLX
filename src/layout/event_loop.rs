use std::io;
use std::path::Path;

use chrono::{DateTime, Local};

use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::engine::db_ops;
use crate::layout::render::draw_ublx_frame;
use crate::layout::setup::{
    DeltaViewData, MainMode, RightPaneContent, TuiRow, UblxState, ViewData,
};
use crate::layout::viewing_pane::resolve_right_pane_content;
use crate::ui::input::handle_ublx_input;

/// Compute filtered categories and contents from search + category selection; clamp list
/// selection and reset preview scroll when selection changes.
pub fn build_view_data(
    state: &mut UblxState,
    categories: &[String],
    all_rows: &[TuiRow],
) -> ViewData {
    let filtered_categories: Vec<String> = if state.search_query.trim().is_empty() {
        categories.to_vec()
    } else {
        let q = state.search_query.trim();
        categories
            .iter()
            .filter(|cat| {
                all_rows
                    .iter()
                    .any(|(path, c, _)| c == *cat && (path.contains(q) || c.contains(q)))
            })
            .cloned()
            .collect()
    };
    let category_idx = state.category_state.selected().unwrap_or(0);
    let selected_category = if category_idx == 0 {
        None
    } else {
        filtered_categories
            .get(category_idx - 1)
            .map(String::as_str)
    };
    let contents_rows: Vec<TuiRow> = match selected_category {
        None => all_rows.to_vec(),
        Some(cat) => all_rows
            .iter()
            .filter(|(_, c, _)| c == cat)
            .cloned()
            .collect(),
    };
    let filtered_contents_rows: Vec<TuiRow> = if state.search_query.trim().is_empty() {
        contents_rows
    } else {
        let q = state.search_query.trim();
        contents_rows
            .iter()
            .filter(|(path, category, _)| path.contains(q) || category.contains(q))
            .cloned()
            .collect()
    };
    let category_list_len = 1 + filtered_categories.len();
    if category_list_len > 0 {
        let idx = category_idx.min(category_list_len.saturating_sub(1));
        state.category_state.select(Some(idx));
    }
    let content_len = filtered_contents_rows.len();
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
    ViewData {
        filtered_categories,
        filtered_contents_rows,
        category_list_len,
        content_len,
    }
}

/// Format Unix timestamp in nanoseconds as local date-time (e.g. "2025-02-06 14:30:00").
fn format_timestamp_ns(ns: i64) -> String {
    const NS_PER_S: i64 = 1_000_000_000;
    let secs = ns / NS_PER_S;
    let subsec = ((ns % NS_PER_S) + NS_PER_S) % NS_PER_S;
    match DateTime::from_timestamp(secs, subsec as u32) {
        Some(utc) => {
            let local = utc.with_timezone(&Local);
            local.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        None => format!("{} (invalid)", ns),
    }
}

/// Load delta_log data for Delta mode: overview text (snapshot count + timestamps) and paths per type.
pub fn build_delta_view_data(db_path: &Path) -> DeltaViewData {
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

    DeltaViewData {
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
pub fn clamp_delta_selection(state: &mut UblxState, delta: &DeltaViewData) {
    let cat_idx = state.category_state.selected().unwrap_or(0).min(2);
    state.category_state.select(Some(cat_idx));
    let paths = delta_paths_for_index(delta, cat_idx);
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

fn delta_paths_for_index(delta: &DeltaViewData, cat_idx: usize) -> &Vec<String> {
    match cat_idx {
        0 => &delta.added_paths,
        1 => &delta.mod_paths,
        _ => &delta.removed_paths,
    }
}

/// ViewData for input/navigation when in Delta mode (3 categories, content_len = current type's path count).
pub fn view_data_for_delta_mode(state: &UblxState, delta: &DeltaViewData) -> ViewData {
    let cat_idx = state.category_state.selected().unwrap_or(0).min(2);
    let paths = delta_paths_for_index(delta, cat_idx);
    ViewData {
        filtered_categories: vec!["Added".into(), "Mod".into(), "Removed".into()],
        filtered_contents_rows: paths
            .iter()
            .map(|p| (p.clone(), String::new(), String::new()))
            .collect(),
        category_list_len: 3,
        content_len: paths.len(),
    }
}

/// **Classification of each call per tick:**
/// 1. **View data** — `compute_loop_view`: filter categories/contents by search and category, clamp selection, update preview key.
/// 2. **Right-pane content** — `resolve_right_pane_content`: templates/metadata/writing/viewer for current selection.
/// 3. **Draw** — `draw_ublx_frame`: layout and render categories, contents, right pane, search, help.
/// 4. **Input** — `handle_ublx_input`: poll key, map to action, apply to state; returns true if quit.
pub fn run_ublx(db_path: &Path, dir_to_ublx: &Path) -> io::Result<()> {
    let categories = db_ops::load_snapshot_categories(db_path).unwrap_or_default();
    let all_rows = db_ops::load_snapshot_rows_for_tui(db_path, None).unwrap_or_default();

    let mut state = UblxState::new();

    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    loop {
        let (view, right_content, delta_data) = if state.main_mode == MainMode::Delta {
            let d = build_delta_view_data(db_path);
            clamp_delta_selection(&mut state, &d);
            let view = view_data_for_delta_mode(&state, &d);
            let right_content = RightPaneContent {
                templates: String::new(),
                metadata: None,
                writing: None,
                viewer: None,
            };
            (view, right_content, Some(d))
        } else {
            let view = build_view_data(&mut state, &categories, &all_rows);
            let right_content =
                resolve_right_pane_content(&mut state, dir_to_ublx, &view.filtered_contents_rows);
            (view, right_content, None)
        };
        terminal
            .draw(|f| draw_ublx_frame(f, &mut state, &view, &right_content, delta_data.as_ref()))?;
        if handle_ublx_input(&mut state, &view, &right_content)? {
            break;
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
