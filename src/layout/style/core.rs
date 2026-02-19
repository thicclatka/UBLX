//! Layout constants, splits, theme-derived style trait and wrappers.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use std::rc::Rc;

use crate::layout::themes;
use crate::ui::UI_CONSTANTS;

/// Viewer scrollbar: vertical right, no begin/end symbols (track + thumb only).
pub fn viewer_scrollbar() -> Scrollbar<'static> {
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
}

/// Split a tab row area into [left pad, content, right pad] using [UI_CONSTANTS](super::UI_CONSTANTS).h_pad.
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

/// Vertical layout split. Reusable so call sites don't repeat `Layout::default().direction(Vertical).constraints(...).split(area)`.
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

/// Return a rect inset by [UI_CONSTANTS](super::UI_CONSTANTS).h_pad on left and right. Use for table (or other) content that should have horizontal padding.
pub fn rect_with_h_pad(area: Rect) -> Rect {
    let pad = UI_CONSTANTS.h_pad;
    let width = area.width.saturating_sub(2 * pad);
    Rect {
        x: area.x + pad,
        y: area.y,
        width,
        height: area.height,
    }
}

/// Trait that provides a theme and default implementations for all theme-derived styles.
/// One implementor ([CurrentTheme]) uses the frame's current theme; others could use a fixed theme or snapshot.
pub trait ThemeStyles {
    fn palette() -> &'static themes::Theme
    where
        Self: Sized;

    fn panel_focused() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
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
        Style::default().fg(Self::palette().text)
    }

    fn tab_active() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default()
            .fg(t.tab_active_fg)
            .bg(t.tab_active_bg)
            .add_modifier(Modifier::BOLD)
    }

    fn tab_inactive() -> Style
    where
        Self: Sized,
    {
        Style::default().bg(Self::palette().tab_inactive_bg)
    }

    fn search_text() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default().fg(t.search_text)
    }

    fn hint_text() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default().fg(t.hint).bg(t.popup_bg)
    }

    fn delta_added() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().delta_added)
    }

    fn delta_mod() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().delta_mod)
    }

    fn delta_removed() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().delta_removed)
    }

    fn title_brand() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().title_brand)
    }
}

/// Uses the current frame theme (set at draw start). Implements [ThemeStyles].
pub struct CurrentTheme;

impl ThemeStyles for CurrentTheme {
    fn palette() -> &'static themes::Theme {
        themes::current()
    }
}

/// List item highlight (selected row).
pub fn list_highlight() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// Theme-derived style wrappers: each calls [CurrentTheme] with the same name. Call these from widgets; they use the theme set at frame start.
macro_rules! theme_style_fn {
    ($name:ident) => {
        pub fn $name() -> Style {
            CurrentTheme::$name()
        }
    };
}

/// Expand to [theme_style_fn!] for each name; add new theme style functions here.
macro_rules! theme_style_fn_thru_list {
    ($($name:ident),* $(,)?) => {
        $( theme_style_fn!($name); )*
    };
}

theme_style_fn_thru_list!(
    panel_focused,
    panel_unfocused,
    text_style,
    tab_active,
    tab_inactive,
    search_text,
    hint_text,
    delta_added,
    delta_mod,
    delta_removed,
    title_brand,
);
