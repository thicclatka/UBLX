//! Layout constants, splits, theme-derived style trait and wrappers.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation};
use std::rc::Rc;

use crate::themes;
use crate::ui::UI_CONSTANTS;

/// Viewer scrollbar: vertical right, no begin/end symbols (track + thumb only).
#[must_use]
pub fn viewer_scrollbar() -> Scrollbar<'static> {
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None)
}

/// Split a tab row area into [left pad, content, right pad] using [`UI_CONSTANTS`](super::UI_CONSTANTS).`h_pad`.
#[must_use]
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
#[must_use]
pub fn split_vertical(area: Rect, constraints: &[Constraint]) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
}

/// Centered popup rect: inner content size plus padding (e.g. for borders/title), clamped to area.
#[must_use]
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

/// Return a rect inset by [`UI_CONSTANTS`](super::UI_CONSTANTS).`h_pad` on left and right. Use for table (or other) content that should have horizontal padding.
#[must_use]
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
/// One implementor ([`CurrentTheme`]) uses the frame's current theme; others could use a fixed theme or snapshot.
pub trait ThemeStyles {
    fn palette() -> &'static themes::Palette
    where
        Self: Sized;

    #[must_use]
    fn panel_focused() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default()
            .fg(t.focused_border)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    fn panel_unfocused() -> Style
    where
        Self: Sized,
    {
        Style::default()
    }

    #[must_use]
    fn text_style() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().text)
    }

    #[must_use]
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

    #[must_use]
    fn tab_inactive() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default().fg(t.text).bg(t.tab_inactive_bg)
    }

    #[must_use]
    fn search_text() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default().fg(t.search_text)
    }

    #[must_use]
    fn hint_text() -> Style
    where
        Self: Sized,
    {
        let t = Self::palette();
        Style::default().fg(t.hint).bg(t.popup_bg)
    }

    #[must_use]
    fn delta_added() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().delta_added)
    }

    #[must_use]
    fn delta_mod() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().delta_mod)
    }

    #[must_use]
    fn delta_removed() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().delta_removed)
    }

    #[must_use]
    fn title_brand() -> Style
    where
        Self: Sized,
    {
        Style::default().fg(Self::palette().title_brand)
    }
}

/// Uses the current frame theme (set at draw start). Implements [`ThemeStyles`].
pub struct CurrentTheme;

impl ThemeStyles for CurrentTheme {
    fn palette() -> &'static themes::Palette {
        themes::current()
    }
}

/// List item highlight (selected row).
#[must_use]
pub fn list_highlight() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

/// List body text when this pane has keyboard focus: bold + theme text color.
#[must_use]
pub fn panel_list_style(pane_focused: bool) -> Style {
    let mut s = CurrentTheme::text_style();
    if pane_focused {
        s = s.add_modifier(Modifier::BOLD);
    }
    s
}

/// Block title line: match border emphasis when focused.
#[must_use]
pub fn panel_title_style(focused: bool) -> Style {
    if focused {
        CurrentTheme::panel_focused()
    } else {
        CurrentTheme::text_style()
    }
}

/// Palette-derived style wrappers: each calls [`CurrentTheme`] with the same name. Call these from widgets; they use the palette set at frame start.
macro_rules! theme_style_fn {
    ($name:ident) => {
        pub fn $name() -> Style {
            CurrentTheme::$name()
        }
    };
}

/// Expand to [`theme_style_fn`!] for each name; add new theme style functions here.
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

/// Non-current in-pane find matches (underlined accent).
#[must_use]
pub fn viewer_find_match() -> Style {
    CurrentTheme::search_text().add_modifier(Modifier::UNDERLINED)
}

/// KV / sheet table cells: body text, bold + underline (no fill; row stripe shows through).
#[must_use]
pub fn viewer_find_match_table_cell() -> Style {
    let t = CurrentTheme::palette();
    Style::default()
        .fg(t.text)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

/// Active find match inside a KV / sheet cell: accent fg (not [`Modifier::REVERSED`]) so it reads on stripes.
#[must_use]
pub fn viewer_find_match_current_table_cell() -> Style {
    let t = CurrentTheme::palette();
    Style::default()
        .fg(t.search_text)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

/// Current in-pane find match.
#[must_use]
pub fn viewer_find_match_current() -> Style {
    CurrentTheme::text_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
}
