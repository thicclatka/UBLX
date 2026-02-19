use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, ScrollbarState, Wrap};

use super::consts::UiStrings;
use super::formatters::markdown::is_markdown_path;
use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::layout::style;
use crate::utils::format_bytes;

const UI: UiStrings = UiStrings::new();

/// Approximate number of wrapped lines for viewer text at the given width (for scroll clamping).
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

/// Clamp preview_scroll so we don't scroll past content (which would blank the viewer).
fn clamped_preview_scroll(scroll: u16, viewer_text: Option<&str>, width: u16, height: u16) -> u16 {
    let Some(text) = viewer_text else {
        return 0;
    };
    let total = wrapped_line_count(text, width);
    let max_scroll = total.saturating_sub(height);
    scroll.min(max_scroll)
}

/// Viewer tab only: if markdown, return styled [ratatui::text::Text] (e.g. nice headers); else plain string.
/// `content_width` is used for full-width horizontal rules when rendering markdown.
fn viewer_display_text(
    right: &RightPaneContent,
    content_width: u16,
) -> ratatui::text::Text<'static> {
    let raw = right.viewer.as_deref().unwrap_or(UI.viewer_placeholder);
    if let Some(ref path) = right.viewer_path
        && is_markdown_path(path)
    {
        return super::formatters::markdown::parse_markdown(raw).to_text(content_width);
    }
    ratatui::text::Text::from(raw.to_string())
}

/// Returns the content to display for the current tab as [ratatui::text::Text].
fn content_display_text(
    state: &UblxState,
    right: &RightPaneContent,
    content_width: u16,
) -> ratatui::text::Text<'static> {
    match state.right_pane_mode {
        RightPaneMode::Viewer => viewer_display_text(right, content_width),
        RightPaneMode::Templates => ratatui::text::Text::from(right.templates.clone()),
        RightPaneMode::Metadata => ratatui::text::Text::from(
            right
                .metadata
                .clone()
                .unwrap_or_else(|| UI.not_available.to_string()),
        ),
        RightPaneMode::Writing => ratatui::text::Text::from(
            right
                .writing
                .clone()
                .unwrap_or_else(|| UI.not_available.to_string()),
        ),
    }
}

fn title(state: &UblxState) -> &'static str {
    match state.right_pane_mode {
        RightPaneMode::Viewer => UI.viewer,
        RightPaneMode::Templates => UI.templates,
        RightPaneMode::Metadata => UI.metadata,
        RightPaneMode::Writing => UI.writing,
    }
}

fn visible_tabs(right: &RightPaneContent) -> Vec<(RightPaneMode, &'static str)> {
    [
        (RightPaneMode::Viewer, UI.tab_viewer),
        (RightPaneMode::Templates, UI.tab_templates),
        (RightPaneMode::Metadata, UI.tab_metadata),
        (RightPaneMode::Writing, UI.tab_writing),
    ]
    .into_iter()
    .filter(|(mode, _)| match mode {
        RightPaneMode::Templates | RightPaneMode::Viewer => true,
        RightPaneMode::Metadata => right.metadata.is_some(),
        RightPaneMode::Writing => right.writing.is_some(),
    })
    .collect()
}

pub(super) fn draw_right_pane(
    f: &mut Frame,
    state: &mut UblxState,
    right: &RightPaneContent,
    area: Rect,
) {
    let show_footer = state.right_pane_mode == RightPaneMode::Viewer
        && (right.viewer_byte_size.is_some() || right.viewer_mtime_ns.is_some());
    let size_str = right.viewer_byte_size.map(format_bytes);
    let footer_line = show_footer
        .then(|| style::viewer_footer_line(size_str.as_deref(), right.viewer_mtime_ns))
        .flatten();
    let right_block = if let Some(ref line) = footer_line {
        Block::default()
            .borders(Borders::ALL)
            .title(title(state))
            .title_bottom(line.clone())
    } else {
        Block::default().borders(Borders::ALL).title(title(state))
    };
    let tabs = visible_tabs(right);
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

    let content_rect = content_chunks[1];
    let show_scrollbar = state.right_pane_mode == RightPaneMode::Viewer;
    let (text_rect, scrollbar_rect) = if show_scrollbar && content_rect.width > 1 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(content_rect);
        (chunks[0], chunks[1])
    } else {
        (content_rect, Rect::default())
    };
    let scroll_y = clamped_preview_scroll(
        state.preview_scroll,
        right.viewer.as_deref(),
        text_rect.width,
        text_rect.height,
    );
    if state.right_pane_mode == RightPaneMode::Viewer && state.preview_scroll > scroll_y {
        state.preview_scroll = scroll_y;
    }
    f.render_widget(
        Paragraph::new(content_display_text(state, right, text_rect.width))
            .style(style::text_style())
            .wrap(Wrap { trim: false })
            .scroll((scroll_y, 0)),
        text_rect,
    );
    if show_scrollbar && scrollbar_rect.width > 0 && scrollbar_rect.height > 0 {
        let total = right
            .viewer
            .as_deref()
            .map(|t| wrapped_line_count(t, text_rect.width) as usize)
            .unwrap_or(0);
        let viewport = text_rect.height as usize;
        let max_scroll = total.saturating_sub(viewport);
        if max_scroll > 0 {
            let content_len = max_scroll + 1;
            let mut scrollbar_state = ScrollbarState::new(content_len)
                .position(scroll_y as usize)
                .viewport_content_length(1);
            f.render_stateful_widget(style::viewer_scrollbar(), scrollbar_rect, &mut scrollbar_state);
        }
    }
}

/// Draw the viewer tab content in full screen (hide categories and contents). Esc to exit.
pub(super) fn draw_viewer_fullscreen(
    f: &mut Frame,
    state: &mut UblxState,
    right: &RightPaneContent,
    area: Rect,
) {
    let size_str = right.viewer_byte_size.map(format_bytes);
    let footer_line = style::viewer_footer_line(size_str.as_deref(), right.viewer_mtime_ns);
    let block = if let Some(line) = footer_line {
        Block::default()
            .borders(Borders::ALL)
            .title(" Viewer (Esc to exit fullscreen) ")
            .title_bottom(line)
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title(" Viewer (Esc to exit fullscreen) ")
    };
    let inner = block.inner(area);
    let (text_rect, scrollbar_rect) = if inner.width > 1 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);
        (chunks[0], chunks[1])
    } else {
        (inner, Rect::default())
    };
    let viewer_content = viewer_display_text(right, text_rect.width);
    let scroll_y = clamped_preview_scroll(
        state.preview_scroll,
        right.viewer.as_deref(),
        text_rect.width,
        text_rect.height,
    );
    if state.preview_scroll > scroll_y {
        state.preview_scroll = scroll_y;
    }
    f.render_widget(&block, area);
    f.render_widget(
        Paragraph::new(viewer_content)
            .style(style::text_style())
            .wrap(Wrap { trim: false })
            .scroll((scroll_y, 0)),
        text_rect,
    );
    let total = right
        .viewer
        .as_deref()
        .map(|t| wrapped_line_count(t, text_rect.width) as usize)
        .unwrap_or(0);
    let viewport = text_rect.height as usize;
    let max_scroll = total.saturating_sub(viewport);
    if scrollbar_rect.width > 0 && scrollbar_rect.height > 0 && max_scroll > 0 {
        let content_len = max_scroll + 1;
        let mut scrollbar_state = ScrollbarState::new(content_len)
            .position(scroll_y as usize)
            .viewport_content_length(1);
        f.render_stateful_widget(style::viewer_scrollbar(), scrollbar_rect, &mut scrollbar_state);
    }
}
