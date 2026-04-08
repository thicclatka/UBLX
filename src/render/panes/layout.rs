//! Layout splits for panel areas.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem};
use unicode_width::UnicodeWidthStr;

use crate::config::LayoutOverlay;
use crate::layout::style;
use crate::ui::{UI_CONSTANTS, UI_STRINGS};

/// Horizontal widths of the three main panes (same split as [`crate::render::core::compute_body_areas`]).
#[must_use]
pub fn three_pane_chunk_widths(term_width: u16, layout: &LayoutOverlay) -> [u16; 3] {
    let main = Rect::new(0, 0, term_width, 1);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout.left_pct),
            Constraint::Percentage(layout.middle_pct),
            Constraint::Percentage(layout.right_pct),
        ])
        .split(main);
    [chunks[0].width, chunks[1].width, chunks[2].width]
}

/// Max display width for text in a bordered list using [`styled_list`] (inner width minus List highlight column).
#[must_use]
pub fn list_row_text_max_cols(outer_panel_width: u16) -> usize {
    let block_inner = outer_panel_width.saturating_sub(2);
    let sym = UnicodeWidthStr::width(UI_STRINGS.list.list_symbol);
    block_inner.saturating_sub(sym as u16) as usize
}

/// Split content area into main area and one status line (Last Snapshot + Search:).
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
