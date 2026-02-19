//! Powerline-style nodes: tabs, footer lines, status spans.

use ratatui::layout::HorizontalAlignment;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::utils::UI_GLYPHS;

use super::{CurrentTheme, ThemeStyles, tab_active, tab_inactive};

/// One tab as a powerline-style node: round + " label " + round. No separator.
/// Used for right-pane tabs and main (Snapshot/Delta) tabs.
pub fn tab_node_segment(label: &str, active: bool) -> Vec<Span<'static>> {
    let (circle_style, node_style) = if active {
        (
            Style::default().fg(CurrentTheme::palette().tab_active_bg),
            tab_active(),
        )
    } else {
        (
            Style::default().fg(CurrentTheme::palette().tab_inactive_bg),
            tab_inactive(),
        )
    };
    vec![
        Span::styled(UI_GLYPHS.round_left.to_string(), circle_style),
        Span::styled(format!(" {} ", label), node_style),
        Span::styled(UI_GLYPHS.round_right.to_string(), circle_style),
    ]
}

fn node_color() -> (ratatui::style::Color, Style, Style) {
    let t = CurrentTheme::palette();
    let circle_style = Style::default().fg(t.node_bg).bg(t.background);
    let node_style = Style::default().bg(t.node_bg);
    (t.node_bg, circle_style, node_style)
}

fn node_spans(content: &str, circle_style: Style, node_style: Style) -> Vec<Span<'static>> {
    vec![
        Span::styled(UI_GLYPHS.round_left.to_string(), circle_style),
        Span::styled(format!(" {} ", content), node_style),
        Span::styled(UI_GLYPHS.round_right.to_string(), circle_style),
    ]
}

/// Single-node footer line with the given alignment (e.g. viewer size = Right, categories "Latest Snapshot" = Left).
pub fn node_line(text: &str, alignment: HorizontalAlignment) -> Line<'static> {
    let (_, circle_style, node_style) = node_color();
    Line::from(node_spans(text, circle_style, node_style)).alignment(alignment)
}

/// Powerline-style node spans for use in a combined status line (e.g. Latest Snapshot, not in a border).
pub fn status_node_spans(content: &str) -> Vec<Span<'static>> {
    let (_, circle_style, node_style) = node_color();
    node_spans(content, circle_style, node_style)
}

/// Footer line with optional size and optional mtime (viewer: byte size + last-modified).
pub fn viewer_footer_line(size_str: Option<&str>, mtime_ns: Option<i64>) -> Option<Line<'static>> {
    use crate::utils::format_timestamp_ns;
    match (size_str, mtime_ns) {
        (Some(s), None) => Some(node_line(s, HorizontalAlignment::Right)),
        (None, Some(ns)) => Some(node_line(
            &format_timestamp_ns(ns),
            HorizontalAlignment::Right,
        )),
        (Some(s), Some(ns)) => {
            let (_, circle_style, node_style) = node_color();
            let spans: Vec<Span<'static>> = [
                node_spans(s, circle_style, node_style),
                node_spans(&format_timestamp_ns(ns), circle_style, node_style),
            ]
            .into_iter()
            .flatten()
            .collect();
            Some(Line::from(spans).right_aligned())
        }
        (None, None) => None,
    }
}
