//! Shared TUI styles and layout constants for the 3-panel layout.
//! Colors come from the current theme ([crate::layout::themes::current]); set at frame start via [crate::layout::themes::set_current].

use ratatui::layout::{Constraint, Direction, HorizontalAlignment, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use std::rc::Rc;

use crate::layout::themes;

/// Current theme for this frame. Style functions use this instead of calling [themes::current] in each.
fn theme() -> &'static themes::Theme {
    themes::current()
}

/// Powerline-style round segment characters.
const ROUND_LEFT: char = '\u{e0b6}';
const ROUND_RIGHT: char = '\u{e0b4}';

/// Viewer scrollbar: vertical right, no begin/end symbols (track + thumb only).
pub fn viewer_scrollbar() -> Scrollbar<'static> {
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
}

/// Shared UI layout constants (padding, etc.).
pub struct UiConstants {
    pub h_pad: u16,
}

impl UiConstants {
    pub const fn new() -> Self {
        Self { h_pad: 1 }
    }
}

pub const UI_CONSTANTS: UiConstants = UiConstants::new();

/// Split a tab row area into [left pad, content, right pad] using [UI_CONSTANTS](self::UI_CONSTANTS).h_pad.
pub fn tab_row_padded(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(UI_CONSTANTS.h_pad),
            Constraint::Min(0),
            Constraint::Length(UI_CONSTANTS.h_pad),
        ])
        .split(area)
}

/// Vertical layout split. Reusable so call sites don’t repeat `Layout::default().direction(Vertical).constraints(...).split(area)`.
pub fn split_vertical(area: Rect, constraints: &[Constraint]) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
}

/// List item highlight (selected row).
pub fn list_highlight() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// Focused panel border (categories or contents).
pub fn panel_focused() -> Style {
    Style::default()
        .fg(theme().focused_border)
        .add_modifier(Modifier::BOLD)
}

/// Unfocused panel border.
pub fn panel_unfocused() -> Style {
    Style::default()
}

/// Default foreground for body text (lists, paragraphs, search). Use when no other style is set.
pub fn text_style() -> Style {
    Style::default().fg(theme().text)
}

/// Active tab (main Snapshot/Delta or right-pane Templates/Viewer etc.): foreground + background.
pub fn tab_active() -> Style {
    Style::default()
        .fg(theme().tab_active_fg)
        .bg(theme().tab_active_bg)
        .add_modifier(Modifier::BOLD)
}

/// Inactive tab: same text as default, only background differs.
pub fn tab_inactive() -> Style {
    Style::default().bg(theme().tab_inactive_bg)
}

/// "Search:" label (and query) in status line.
pub fn search_text() -> Style {
    Style::default().fg(theme().search_text)
}

/// "Esc to clear" hint text at bottom.
pub fn hint_text() -> Style {
    Style::default().fg(theme().hint)
}

/// Delta left-pane: added (green).
pub fn delta_added() -> Style {
    Style::default().fg(theme().delta_added)
}

/// Delta left-pane: mod (yellow).
pub fn delta_mod() -> Style {
    Style::default().fg(theme().delta_mod)
}

/// Delta left-pane: removed (red).
pub fn delta_removed() -> Style {
    Style::default().fg(theme().delta_removed)
}

/// Top-right brand title (e.g. UBLX).
pub fn title_brand() -> Style {
    Style::default().fg(theme().title_brand)
}

// ---- Tab and footer nodes (powerline-style) ----

/// One tab as a powerline-style node: round + " label " + round. No separator.
/// Used for right-pane tabs and main (Snapshot/Delta) tabs.
pub fn tab_node_segment(label: &str, active: bool) -> Vec<Span<'static>> {
    let (circle_style, node_style) = if active {
        (Style::default().fg(theme().tab_active_bg), tab_active())
    } else {
        (Style::default().fg(theme().tab_inactive_bg), tab_inactive())
    };
    vec![
        Span::styled(ROUND_LEFT.to_string(), circle_style),
        Span::styled(format!(" {} ", label), node_style),
        Span::styled(ROUND_RIGHT.to_string(), circle_style),
    ]
}

fn node_color() -> (ratatui::style::Color, Style, Style) {
    let t = theme();
    let circle_style = Style::default().fg(t.node_bg).bg(t.node_circle_bg);
    let node_style = Style::default().bg(t.node_bg);
    (t.node_bg, circle_style, node_style)
}

fn node_spans(content: &str, circle_style: Style, node_style: Style) -> Vec<Span<'static>> {
    vec![
        Span::styled(ROUND_LEFT.to_string(), circle_style),
        Span::styled(format!(" {} ", content), node_style),
        Span::styled(ROUND_RIGHT.to_string(), circle_style),
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
