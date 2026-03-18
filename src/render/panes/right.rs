//! Right pane: tabs, viewer/templates/metadata/writing content, fullscreen.

use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::layout::style;
use crate::render::viewers::markdown::{is_markdown_path, parse_markdown};
use crate::render::{kv_tables, scrollable_content};
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::format::StringObjTraits;
use crate::utils::format_bytes;

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
    right_content: &RightPaneContent,
    content_width: u16,
) -> ratatui::text::Text<'static> {
    let raw = right_content
        .viewer
        .as_deref()
        .unwrap_or(UI_STRINGS.viewer_placeholder);
    if let Some(ref path) = right_content.viewer_path
        && is_markdown_path(path)
    {
        return parse_markdown(raw).to_text(content_width);
    }
    ratatui::text::Text::from(raw.to_string())
}

fn content_display_text(
    state: &UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) -> ratatui::text::Text<'static> {
    match state.right_pane_mode {
        RightPaneMode::Viewer => viewer_display_text(right_content, content_width),
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
    let use_kv_tables = match state.right_pane_mode {
        RightPaneMode::Metadata => right_content.metadata.as_deref(),
        RightPaneMode::Writing => right_content.writing.as_deref(),
        _ => None,
    };
    let total_lines = match (state.right_pane_mode, use_kv_tables) {
        (_, Some(json)) => kv_tables::content_height(json) as usize,
        (RightPaneMode::Viewer, _) => right_content
            .viewer
            .as_deref()
            .map(|t| wrapped_line_count(t, content_area.width) as usize)
            .unwrap_or(0),
        (RightPaneMode::Templates, _) => right_content.templates.lines().count(),
        (RightPaneMode::Writing, _) | (RightPaneMode::Metadata, _) => 0,
    };
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
        f.render_widget(
            Paragraph::new(content_display_text(state, right_content, text_rect.width))
                .style(style::text_style())
                .wrap(Wrap { trim: false })
                .scroll((layout.scroll_y, 0)),
            text_rect,
        );
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
    let use_kv_tables = match state.right_pane_mode {
        RightPaneMode::Metadata => right_content.metadata.as_deref(),
        RightPaneMode::Writing => right_content.writing.as_deref(),
        _ => None,
    };
    let total_lines = match (state.right_pane_mode, use_kv_tables) {
        (_, Some(json)) => kv_tables::content_height(json) as usize,
        (RightPaneMode::Viewer, _) => right_content
            .viewer
            .as_deref()
            .map(|t| wrapped_line_count(t, inner.width) as usize)
            .unwrap_or(0),
        (RightPaneMode::Templates, _) => right_content.templates.lines().count(),
        (RightPaneMode::Writing, _) | (RightPaneMode::Metadata, _) => 0,
    };
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
        f.render_widget(
            Paragraph::new(content_display_text(state, right_content, text_rect.width))
                .style(style::text_style())
                .wrap(Wrap { trim: false })
                .scroll((layout.scroll_y, 0)),
            text_rect,
        );
    }
    scrollable_content::draw_scrollbar(f, &layout, total_lines);
}
