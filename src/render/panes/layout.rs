//! Layout splits for panel areas.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem};

use crate::layout::style;
use crate::ui::{UI_CONSTANTS, UI_STRINGS};

/// Split content area into main area and one status line (Latest Snapshot + Search:).
#[must_use]
pub fn split_main_and_status(content_area: Rect) -> (Rect, Rect) {
    let vertical = style::split_vertical(content_area, &UI_CONSTANTS.status_line_constraints());
    (vertical[0], vertical[1])
}

/// Builds a panel block with borders and title.
pub fn panel_block<'a, T: Into<ratatui::text::Line<'a>>>(title: T, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title_style(Style::default())
        .title(title)
}

/// Same powerline node as the main tab bar ([`style::tab_node_segment`]).
#[must_use]
pub fn panel_title_line(label: &str, focused: bool, chord_mode: bool) -> Line<'static> {
    Line::from(style::tab_node_segment(label, focused, chord_mode)).left_aligned()
}

/// Builds a styled list with items, block, and highlight style.
#[must_use]
pub fn styled_list<'a>(
    items: Vec<ListItem<'a>>,
    block: Block<'a>,
    highlight_style: ratatui::style::Style,
    pane_focused: bool,
) -> List<'a> {
    List::new(items)
        .block(block)
        .style(style::panel_list_style(pane_focused))
        .highlight_style(highlight_style)
        .highlight_symbol(UI_STRINGS.list.list_symbol)
        .highlight_spacing(HighlightSpacing::Always)
}

/// Draws a list panel with items, block, highlight style, and list state.
pub fn draw_list_panel(
    f: &mut ratatui::Frame,
    items: Vec<ListItem>,
    block: Block,
    highlight_style: ratatui::style::Style,
    pane_focused: bool,
    list_state: &mut ratatui::widgets::ListState,
    area: Rect,
) {
    f.render_stateful_widget(
        styled_list(items, block, highlight_style, pane_focused),
        area,
        list_state,
    );
}
