//! Shared TUI styles and layout constants for the 3-panel layout.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use std::rc::Rc;

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
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Unfocused panel border.
pub fn panel_unfocused() -> Style {
    Style::default()
}

/// Active tab (main Snapshot/Delta or right-pane Templates/Viewer etc.): foreground + background.
pub fn tab_active() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .bg(Color::Rgb(70, 70, 90))
        .add_modifier(Modifier::BOLD)
}

/// Inactive tab: same text as default, only background differs.
pub fn tab_inactive() -> Style {
    Style::default().bg(Color::Rgb(45, 45, 45))
}

/// Search bar block border.
pub fn search_border() -> Style {
    Style::default().fg(Color::Yellow)
}

/// "Esc to clear" hint text at bottom.
pub fn hint_text() -> Style {
    Style::default().fg(Color::Cyan)
}

/// Delta left-pane: added (green).
pub fn delta_added() -> Style {
    Style::default().fg(Color::Green)
}

/// Delta left-pane: mod (yellow).
pub fn delta_mod() -> Style {
    Style::default().fg(Color::Yellow)
}

/// Delta left-pane: removed (red).
pub fn delta_removed() -> Style {
    Style::default().fg(Color::Red)
}

/// Top-right brand title (e.g. UBLX).
pub fn title_brand() -> Style {
    Style::default().fg(Color::Magenta)
}

