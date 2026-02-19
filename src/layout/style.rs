//! Shared TUI styles and layout constants for the 3-panel layout.
//! Colors come from the current theme ([crate::layout::themes::current]); set at frame start via [crate::layout::themes::set_current].

use ratatui::layout::{Constraint, Direction, HorizontalAlignment, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use std::rc::Rc;

use crate::layout::themes;
use crate::utils::UI_GLYPHS;

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

/// Centered popup rect: inner content size plus padding (e.g. for borders/title), clamped to area.
pub fn centered_popup_rect(
    area: Rect,
    content_w: usize,
    content_h: usize,
    padding_w: u16,
    padding_h: u16,
) -> Rect {
    let w = (content_w + usize::from(padding_w)).min(area.width as usize) as u16;
    let h = (content_h + usize::from(padding_h)).min(area.height as usize) as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(x, y, w, h)
}

/// Trait that provides a theme and default implementations for all theme-derived styles.
/// One implementor ([Current]) uses the frame's current theme; others could use a fixed theme or snapshot.
pub trait ThemeStyles {
    fn theme() -> &'static themes::Theme
    where
        Self: Sized;

    fn panel_focused() -> Style
    where
        Self: Sized,
    {
        let t = Self::theme();
        Style::default()
            .fg(t.focused_border)
            .add_modifier(Modifier::BOLD)
    }

    fn panel_unfocused() -> Style
    where
        Self: Sized,
    {
        Style::default()
    }

    fn text_style() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::theme().text)
    }

    fn tab_active() -> Style
    where
        Self: Sized,
    {
        let t = Self::theme();
        Style::default()
            .fg(t.tab_active_fg)
            .bg(t.tab_active_bg)
            .add_modifier(Modifier::BOLD)
    }

    fn tab_inactive() -> Style
    where
        Self: Sized,
    {
        Style::default().bg(Self::theme().tab_inactive_bg)
    }

    fn search_text() -> Style
    where
        Self: Sized,
    {
        let t = Self::theme();
        Style::default().fg(t.search_text)
    }

    fn hint_text() -> Style
    where
        Self: Sized,
    {
        let t = Self::theme();
        Style::default().fg(t.hint).bg(t.popup_bg)
    }

    fn delta_added() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::theme().delta_added)
    }

    fn delta_mod() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::theme().delta_mod)
    }

    fn delta_removed() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::theme().delta_removed)
    }

    fn title_brand() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::theme().title_brand)
    }
}

/// Uses the current frame theme (set at draw start). Implements [ThemeStyles].
pub struct Current;

impl ThemeStyles for Current {
    fn theme() -> &'static themes::Theme {
        themes::current()
    }
}

/// List item highlight (selected row).
pub fn list_highlight() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// Theme-derived style wrappers: each calls [Current] with the same name. Call these from widgets; they use the theme set at frame start.
macro_rules! theme_style_fn {
    ($name:ident) => {
        pub fn $name() -> Style {
            Current::$name()
        }
    };
}

theme_style_fn!(panel_focused);
theme_style_fn!(panel_unfocused);
theme_style_fn!(text_style);
theme_style_fn!(tab_active);
theme_style_fn!(tab_inactive);
theme_style_fn!(search_text);
theme_style_fn!(hint_text);
theme_style_fn!(delta_added);
theme_style_fn!(delta_mod);
theme_style_fn!(delta_removed);
theme_style_fn!(title_brand);

/// One tab as a powerline-style node: round + " label " + round. No separator.
/// Used for right-pane tabs and main (Snapshot/Delta) tabs.
pub fn tab_node_segment(label: &str, active: bool) -> Vec<Span<'static>> {
    let (circle_style, node_style) = if active {
        (
            Style::default().fg(Current::theme().tab_active_bg),
            tab_active(),
        )
    } else {
        (
            Style::default().fg(Current::theme().tab_inactive_bg),
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
    let t = Current::theme();
    // Circle: node_bg for the curve, background for the rest so the half-circle shape is visible.
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
