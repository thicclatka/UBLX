//! Right pane: tabs, viewer / templates / metadata / writing content, fullscreen.
//!
//! ## Viewer pipeline (width, wrap, cache)
//!
//! 1. **Text width** — After vertical padding, [`scrollable_content::area_above_bottom_pad`], we
//!    iterate (up to 6 times): estimate total lines at width `text_w` →
//!    [`scrollable_content::viewport_text_width`] may shrink width to reserve space for a vertical
//!    scrollbar → repeat until stable. All layout and line counting for the viewer must use this
//!    final `text_w`.
//! 2. **CSV cache** — [`ensure_viewer_csv_cache`] fills [`UblxState::viewer_csv_cache`] for viewer
//!    mode + CSV paths: `(path, content_width)` → prebuilt table string + line count so we do not
//!    parse/build more than once per frame when width is stable.
//! 3. **Total lines** — [`viewer_total_lines`] must match what is actually drawn (scrollbar height).
//!    Branches: JSON metadata/writing → [`kv_tables::content_height`]; CSV → cache or
//!    [`csv::table_line_count`]; otherwise plain text → [`wrapped_line_count`].
//! 4. **Scroll** — [`scrollable_content::layout_scrollable_content`] + [`scrollable_content::draw_scrollbar`].
//! 5. **Draw** — If metadata/writing JSON is present, [`kv_tables::draw_tables`] (no `Paragraph`).
//!    Else [`content_display_text`] → `Paragraph` with optional ratatui [`Wrap`].
//!
//! **Preformatted layout:** [`viewer_uses_preformatted_layout`] is true for Markdown and for CSV
//! that parsed to a non-empty table. Those paths already produce viewport-width lines; ratatui must
//! *not* wrap again ([`ratatui_wrap_right_paragraph`]). Plain text and failed/empty CSV still use
//! `Wrap`.
//!
//! ## Adding another path-based viewer
//!
//! Keep display, line count, and wrap flag aligned: extend [`viewer_display_text`] and
//! [`viewer_total_lines`], and set whether [`viewer_uses_preformatted_layout`] should skip `Wrap`.
//! Add a cache in state only if you need `(path, width)` reuse like CSV.

use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::layout::style;
use crate::render::viewers::{csv, markdown};
use crate::render::{kv_tables, scrollable_content};
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::{format::StringObjTraits, format_bytes};

/// Ensure CSV viewer cache is filled for current (path, width) so we parse/build only once per frame.
fn ensure_viewer_csv_cache(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) {
    if state.right_pane_mode != RightPaneMode::Viewer {
        return;
    }
    let Some(path) = right_content.viewer_path.as_deref() else {
        state.viewer_csv_cache = None;
        return;
    };
    if !csv::is_csv_path(path) {
        state.viewer_csv_cache = None;
        return;
    }
    let Some(raw) = right_content.viewer.as_deref() else {
        state.viewer_csv_cache = None;
        return;
    };
    if let Some(ref c) = state.viewer_csv_cache
        && c.0 == path
        && c.1 == content_width
    {
        return;
    }
    match csv::parse_csv(raw) {
        Ok(rows) if !rows.is_empty() => {
            let (table_string, line_count) = csv::table_string_and_line_count(&rows, content_width);
            state.viewer_csv_cache =
                Some((path.to_string(), content_width, table_string, line_count));
        }
        _ => state.viewer_csv_cache = None,
    }
}

/// Total line count for the right-pane content (used for scroll height).
fn viewer_total_lines(
    right_content: &RightPaneContent,
    content_width: u16,
    use_kv_tables: Option<&str>,
    state: &UblxState,
) -> usize {
    match (state.right_pane_mode, use_kv_tables) {
        (_, Some(json)) => kv_tables::content_height(json) as usize,
        (RightPaneMode::Viewer, _) => {
            if let Some(ref c) = state.viewer_csv_cache
                && c.0 == right_content.viewer_path.as_deref().unwrap_or("")
                && c.1 == content_width
            {
                return c.3;
            }
            if right_content
                .viewer_path
                .as_deref()
                .is_some_and(csv::is_csv_path)
                && let Some(raw) = right_content.viewer.as_deref()
                && let Ok(rows) = csv::parse_csv(raw)
                && !rows.is_empty()
            {
                return csv::table_line_count(&rows, content_width);
            }
            right_content
                .viewer
                .as_deref()
                .map_or(0, |t| wrapped_line_count(t, content_width) as usize)
        }
        (RightPaneMode::Templates, _) => right_content.templates.lines().count(),
        (RightPaneMode::Writing | RightPaneMode::Metadata, _) => 0,
    }
}

/// Markdown and successful CSV tables are fully laid out to the viewport; ratatui must not re-wrap.
fn viewer_uses_preformatted_layout(state: &UblxState, right_content: &RightPaneContent) -> bool {
    if state.right_pane_mode != RightPaneMode::Viewer {
        return false;
    }
    let Some(path) = right_content.viewer_path.as_deref() else {
        return false;
    };
    if markdown::is_markdown_path(path) {
        return true;
    }
    if !csv::is_csv_path(path) {
        return false;
    }
    let Some(raw) = right_content.viewer.as_deref() else {
        return false;
    };
    csv::parse_csv(raw).map(|r| !r.is_empty()).unwrap_or(false)
}

fn ratatui_wrap_right_paragraph(state: &UblxState, right_content: &RightPaneContent) -> bool {
    !viewer_uses_preformatted_layout(state, right_content)
}

fn wrapped_line_count(text: &str, width: u16) -> u16 {
    let w = width as usize;
    if w == 0 {
        return 0;
    }
    text.lines()
        .map(|line| (line.chars().count().div_ceil(w)).max(1))
        .sum::<usize>()
        .min(u16::MAX as usize) as u16
}

fn viewer_display_text(
    state: &UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) -> ratatui::text::Text<'static> {
    let raw = right_content
        .viewer
        .as_deref()
        .unwrap_or(UI_STRINGS.viewer_placeholder);
    if let Some(ref path) = right_content.viewer_path {
        if csv::is_csv_path(path) {
            if let Some(ref c) = state.viewer_csv_cache
                && c.0 == path.as_str()
                && c.1 == content_width
            {
                return csv::table_string_to_text(&c.2);
            }
            if let Ok(rows) = csv::parse_csv(raw)
                && !rows.is_empty()
            {
                return csv::table_to_text(&rows, content_width);
            }
            // Parse failed or empty: fall back to raw
        } else if markdown::is_markdown_path(path) {
            return markdown::parse_markdown(raw).to_text(content_width);
        }
    }
    ratatui::text::Text::from(raw.to_string())
}

fn content_display_text(
    state: &UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) -> ratatui::text::Text<'static> {
    match state.right_pane_mode {
        RightPaneMode::Viewer => viewer_display_text(state, right_content, content_width),
        RightPaneMode::Templates => ratatui::text::Text::from(right_content.templates.clone()),
        RightPaneMode::Metadata => ratatui::text::Text::from(
            right_content
                .metadata
                .clone()
                .unwrap_or_else(|| UI_STRINGS.not_available.to_string()),
        ),
        RightPaneMode::Writing => ratatui::text::Text::from(
            right_content
                .writing
                .clone()
                .unwrap_or_else(|| UI_STRINGS.not_available.to_string()),
        ),
    }
}

fn title(state: &UblxState) -> String {
    let label = match state.right_pane_mode {
        RightPaneMode::Viewer => UI_STRINGS.viewer,
        RightPaneMode::Templates => UI_STRINGS.templates,
        RightPaneMode::Metadata => UI_STRINGS.metadata,
        RightPaneMode::Writing => UI_STRINGS.writing,
    };
    UI_STRINGS.pad(label)
}

fn visible_tabs(right_content: &RightPaneContent) -> Vec<(RightPaneMode, &'static str)> {
    [
        (RightPaneMode::Viewer, UI_STRINGS.tab_viewer),
        (RightPaneMode::Templates, UI_STRINGS.tab_templates),
        (RightPaneMode::Metadata, UI_STRINGS.tab_metadata),
        (RightPaneMode::Writing, UI_STRINGS.tab_writing),
    ]
    .into_iter()
    .filter(|(mode, _)| match mode {
        RightPaneMode::Viewer => true,
        RightPaneMode::Templates => !right_content.templates.is_empty(),
        RightPaneMode::Metadata => right_content.metadata.is_some(),
        RightPaneMode::Writing => right_content.writing.is_some(),
    })
    .collect()
}

/// Draw the right (viewer) pane. `chunks` must have at least 3 elements; the right pane uses `chunks[2]`.
pub fn draw_right_pane(
    f: &mut ratatui::Frame,
    state: &mut UblxState,
    right_content: &RightPaneContent,
    chunks: &[Rect],
) {
    let area = chunks[2];
    let show_footer = state.right_pane_mode == RightPaneMode::Viewer
        && (right_content.open_hint_label.is_some()
            || right_content.viewer_byte_size.is_some()
            || right_content.viewer_mtime_ns.is_some());
    let size_str = right_content.viewer_byte_size.map(format_bytes);
    let footer_line = show_footer
        .then(|| {
            style::viewer_footer_line(
                right_content.open_hint_label.as_deref(),
                size_str.as_deref(),
                right_content.viewer_mtime_ns,
            )
        })
        .flatten();
    let block_title = title(state);
    let right_block = if let Some(ref line) = footer_line {
        Block::default()
            .borders(Borders::ALL)
            .title(block_title.as_str())
            .title_bottom(line.clone())
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title(block_title.as_str())
    };
    let tabs = visible_tabs(right_content);
    if !tabs.is_empty() && !tabs.iter().any(|(m, _)| *m == state.right_pane_mode) {
        state.right_pane_mode = tabs[0].0;
    }
    let tab_spans: Vec<Span> = tabs
        .iter()
        .flat_map(|(mode, label)| style::tab_node_segment(label, *mode == state.right_pane_mode))
        .collect();
    let right_inner = right_block.inner(area);
    let constraints = &[
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ][..];
    let right_split = style::split_vertical(right_inner, constraints);
    let tab_row_chunks = style::tab_row_padded(right_split[0]);
    let content_chunks = style::tab_row_padded(right_split[2]);

    f.render_widget(&right_block, area);
    f.render_widget(Paragraph::new(Line::from(tab_spans)), tab_row_chunks[1]);

    let content_area = content_chunks[1];
    let bottom_pad = UI_CONSTANTS.v_pad;
    let padded = scrollable_content::area_above_bottom_pad(content_area, bottom_pad);
    let use_kv_tables = match state.right_pane_mode {
        RightPaneMode::Metadata => right_content.metadata.as_deref(),
        RightPaneMode::Writing => right_content.writing.as_deref(),
        _ => None,
    };
    let mut text_w = padded.width;
    for _ in 0..6 {
        ensure_viewer_csv_cache(state, right_content, text_w);
        let guess_lines = viewer_total_lines(right_content, text_w, use_kv_tables, state);
        let w_next = scrollable_content::viewport_text_width(padded, guess_lines);
        if w_next == text_w {
            break;
        }
        text_w = w_next;
    }
    ensure_viewer_csv_cache(state, right_content, text_w);
    let total_lines = viewer_total_lines(right_content, text_w, use_kv_tables, state);
    let layout = scrollable_content::layout_scrollable_content(
        content_area,
        total_lines,
        &mut state.panels.preview_scroll,
        bottom_pad,
    );
    let text_rect = layout.content_rect;
    if let Some(json) = use_kv_tables {
        kv_tables::draw_tables(f, text_rect, json, layout.scroll_y);
    } else {
        let mut paragraph = Paragraph::new(content_display_text(state, right_content, text_w))
            .style(style::text_style())
            .scroll((layout.scroll_y, 0));
        if ratatui_wrap_right_paragraph(state, right_content) {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        f.render_widget(paragraph, text_rect);
    }
    scrollable_content::draw_scrollbar(f, &layout, total_lines);
}

/// Draw the current right-pane tab in full screen (hide categories and contents). Esc to exit.
pub fn draw_right_pane_fullscreen(
    f: &mut ratatui::Frame,
    state: &mut UblxState,
    right_content: &RightPaneContent,
    area: Rect,
) {
    let show_footer = state.right_pane_mode == RightPaneMode::Viewer
        && (right_content.open_hint_label.is_some()
            || right_content.viewer_byte_size.is_some()
            || right_content.viewer_mtime_ns.is_some());
    let size_str = right_content.viewer_byte_size.map(format_bytes);
    let footer_line = show_footer
        .then(|| {
            style::viewer_footer_line(
                right_content.open_hint_label.as_deref(),
                size_str.as_deref(),
                right_content.viewer_mtime_ns,
            )
        })
        .flatten();
    let fullscreen_title = format!("{} {}", title(state), UI_STRINGS.fullscreen_suffix);
    let block = if let Some(ref line) = footer_line {
        Block::default()
            .borders(Borders::ALL)
            .title(fullscreen_title.as_str())
            .title_bottom(line.clone())
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title(fullscreen_title.as_str())
    };
    let inner = block.inner(area);
    let bottom_pad = UI_CONSTANTS.v_pad;
    let padded = scrollable_content::area_above_bottom_pad(inner, bottom_pad);
    let use_kv_tables = match state.right_pane_mode {
        RightPaneMode::Metadata => right_content.metadata.as_deref(),
        RightPaneMode::Writing => right_content.writing.as_deref(),
        _ => None,
    };
    let mut text_w = padded.width;
    for _ in 0..6 {
        ensure_viewer_csv_cache(state, right_content, text_w);
        let guess_lines = viewer_total_lines(right_content, text_w, use_kv_tables, state);
        let w_next = scrollable_content::viewport_text_width(padded, guess_lines);
        if w_next == text_w {
            break;
        }
        text_w = w_next;
    }
    ensure_viewer_csv_cache(state, right_content, text_w);
    let total_lines = viewer_total_lines(right_content, text_w, use_kv_tables, state);
    let layout = scrollable_content::layout_scrollable_content(
        inner,
        total_lines,
        &mut state.panels.preview_scroll,
        bottom_pad,
    );
    let text_rect = layout.content_rect;
    f.render_widget(&block, area);
    if let Some(json) = use_kv_tables {
        kv_tables::draw_tables(f, text_rect, json, layout.scroll_y);
    } else {
        let mut paragraph = Paragraph::new(content_display_text(state, right_content, text_w))
            .style(style::text_style())
            .scroll((layout.scroll_y, 0));
        if ratatui_wrap_right_paragraph(state, right_content) {
            paragraph = paragraph.wrap(Wrap { trim: false });
        }
        f.render_widget(paragraph, text_rect);
    }
    scrollable_content::draw_scrollbar(f, &layout, total_lines);
}
