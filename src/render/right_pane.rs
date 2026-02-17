use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::consts::UiStrings;
use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::layout::style;

const UI: UiStrings = UiStrings::new();

const ROUND_LEFT: char = '\u{e0b6}';
const ROUND_RIGHT: char = '\u{e0b4}';

/// One tab as a powerline-style node: round + " label " + round. No separator.
/// Used for right-pane tabs and main (Snapshot/Delta) tabs.
pub fn tab_node_segment(label: &str, active: bool) -> Vec<Span<'static>> {
    let (circle_style, node_style) = if active {
        (
            Style::default().fg(Color::Rgb(70, 70, 90)),
            style::tab_active(),
        )
    } else {
        (
            Style::default().fg(Color::Rgb(45, 45, 45)),
            style::tab_inactive(),
        )
    };
    vec![
        Span::styled(ROUND_LEFT.to_string(), circle_style),
        Span::styled(format!(" {} ", label), node_style),
        Span::styled(ROUND_RIGHT.to_string(), circle_style),
    ]
}

fn footer_node_line(size_str: &str) -> Line<'static> {
    let node_color = Color::Rgb(55, 55, 65);
    let circle_style = Style::default().fg(node_color).bg(Color::Black);
    let node_style = Style::default().bg(node_color);
    Line::from(vec![
        Span::styled(ROUND_LEFT.to_string(), circle_style),
        Span::styled(format!(" {} ", size_str), node_style),
        Span::styled(ROUND_RIGHT.to_string(), circle_style),
    ])
    .right_aligned()
}

/// Format byte count as "B", "KB", "MB", "GB" etc.
pub fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n < KB {
        format!("{} B", n)
    } else if n < MB {
        format!("{:.2} KB", n as f64 / KB as f64)
    } else if n < GB {
        format!("{:.2} MB", n as f64 / MB as f64)
    } else {
        format!("{:.2} GB", n as f64 / GB as f64)
    }
}

fn is_markdown_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

/// Viewer tab only: if markdown, return styled [ratatui::text::Text] (e.g. nice headers); else plain string.
/// `content_width` is used for full-width horizontal rules when rendering markdown.
fn viewer_display_text(right: &RightPaneContent, content_width: u16) -> ratatui::text::Text<'static> {
    let raw = right
        .viewer
        .as_deref()
        .unwrap_or(UI.viewer_placeholder);
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
    state: &UblxState,
    right: &RightPaneContent,
    area: Rect,
) {
    let show_size_on_border =
        state.right_pane_mode == RightPaneMode::Viewer && right.viewer_byte_size.is_some();
    let size_str =
        show_size_on_border.then(|| right.viewer_byte_size.map(format_bytes).unwrap_or_default());
    let right_block = if let Some(ref s) = size_str {
        Block::default()
            .borders(Borders::ALL)
            .title(title(state))
            .title_bottom(footer_node_line(s))
    } else {
        Block::default().borders(Borders::ALL).title(title(state))
    };
    let tabs = visible_tabs(right);
    let tab_spans: Vec<Span> = tabs
        .iter()
        .flat_map(|(mode, label)| tab_node_segment(label, *mode == state.right_pane_mode))
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

    f.render_widget(
        Paragraph::new(content_display_text(state, right, content_chunks[1].width))
            .wrap(Wrap { trim: false })
            .scroll((state.preview_scroll, 0)),
        content_chunks[1],
    );
}

/// Draw the viewer tab content in full screen (hide categories and contents). Esc to exit.
pub(super) fn draw_viewer_fullscreen(
    f: &mut Frame,
    state: &UblxState,
    right: &RightPaneContent,
    area: Rect,
) {
    let size_str = right.viewer_byte_size.map(format_bytes);
    let block = match &size_str {
        Some(s) => Block::default()
            .borders(Borders::ALL)
            .title(" Viewer (Esc to exit fullscreen) ")
            .title_bottom(footer_node_line(s)),
        None => Block::default()
            .borders(Borders::ALL)
            .title(" Viewer (Esc to exit fullscreen) "),
    };
    let inner = block.inner(area);
    let viewer_content = viewer_display_text(right, inner.width);
    f.render_widget(&block, area);
    f.render_widget(
        Paragraph::new(viewer_content)
            .wrap(Wrap { trim: false })
            .scroll((state.preview_scroll, 0)),
        inner,
    );
}
