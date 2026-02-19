//! Shared layout and list panel helpers used by snapshot, delta, and core.

use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem};

use crate::layout::style;
use crate::ui::UI_STRINGS;

/// Split content area into main area and one status line (Latest Snapshot + Search:).
pub(super) fn split_main_and_status(content_area: Rect) -> (Rect, Rect) {
    let vertical =
        style::split_vertical(content_area, &[Constraint::Min(1), Constraint::Length(1)]);
    (vertical[0], vertical[1])
}

pub(super) fn panel_block<'a, T: Into<ratatui::text::Line<'a>>>(
    title: T,
    focused: bool,
) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title(title)
}

/// Build a list with standard panel styling (block, highlight, symbol, spacing).
pub(super) fn styled_list<'a>(
    items: Vec<ListItem<'a>>,
    block: Block<'a>,
    focused: bool,
    highlight_style: ratatui::style::Style,
) -> List<'a> {
    let symbol = if focused {
        UI_STRINGS.list_highlight
    } else {
        UI_STRINGS.list_unfocused
    };
    List::new(items)
        .block(block)
        .style(style::text_style())
        .highlight_style(highlight_style)
        .highlight_symbol(symbol)
        .highlight_spacing(HighlightSpacing::Always)
}

pub(super) fn draw_list_panel(
    f: &mut ratatui::Frame,
    items: Vec<ListItem>,
    block: Block,
    focused: bool,
    highlight_style: ratatui::style::Style,
    list_state: &mut ratatui::widgets::ListState,
    area: Rect,
) {
    f.render_stateful_widget(
        styled_list(items, block, focused, highlight_style),
        area,
        list_state,
    );
}

/// Builds a panel block title: `" Label "` or `" ► Label "` when focused.
pub fn set_title(label: &str, focused: bool) -> String {
    if focused {
        format!(" ► {} ", label)
    } else {
        format!(" {} ", label)
    }
}
