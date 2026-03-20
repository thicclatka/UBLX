//! Powerline-style nodes: tabs, footer lines, status spans.

use ratatui::layout::HorizontalAlignment;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::ui::UI_GLYPHS;

use super::{CurrentTheme, ThemeStyles, tab_active, tab_inactive};

/// One tab as a powerline-style node: round + " label " + round. No separator.
/// Used for right-pane tabs and main (Snapshot/Delta) tabs.
#[must_use]
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
        Span::styled(format!(" {label} "), node_style),
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
        Span::styled(format!(" {content} "), node_style),
        Span::styled(UI_GLYPHS.round_right.to_string(), circle_style),
    ]
}

/// Single-node footer line with the given alignment (e.g. viewer size = Right, categories "Latest Snapshot" = Left).
#[must_use]
pub fn node_line(text: &str, alignment: HorizontalAlignment) -> Line<'static> {
    let (_, circle_style, node_style) = node_color();
    Line::from(node_spans(text, circle_style, node_style)).alignment(alignment)
}

/// Powerline-style node spans for use in a combined status line (e.g. Latest Snapshot, not in a border).
#[must_use]
pub fn status_node_spans(content: &str) -> Vec<Span<'static>> {
    let (_, circle_style, node_style) = node_color();
    node_spans(content, circle_style, node_style)
}

/// Footer line with optional open hint (↗ or ↗ (Terminal)/(GUI)), optional size, and optional mtime.
#[must_use]
pub fn viewer_footer_line(
    open_hint_label: Option<&str>,
    size_str: Option<&str>,
    mtime_ns: Option<i64>,
) -> Option<Line<'static>> {
    use crate::utils::format_timestamp_ns;
    let (_, circle_style, node_style) = node_color();
    let mut spans: Vec<Span<'static>> = Vec::new();
    if let Some(label) = open_hint_label {
        spans.extend(node_spans(label, circle_style, node_style));
    }
    match (size_str, mtime_ns) {
        (Some(s), None) => {
            spans.extend(node_spans(s, circle_style, node_style));
        }
        (None, Some(ns)) => {
            spans.extend(node_spans(
                &format_timestamp_ns(ns),
                circle_style,
                node_style,
            ));
        }
        (Some(s), Some(ns)) => {
            spans.extend(node_spans(s, circle_style, node_style));
            spans.extend(node_spans(
                &format_timestamp_ns(ns),
                circle_style,
                node_style,
            ));
        }
        (None, None) if open_hint_label.is_none() => return None,
        (None, None) => {}
    }
    Some(Line::from(spans).right_aligned())
}
