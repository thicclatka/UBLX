//! Styled list panel drawing.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, HighlightSpacing, List, ListItem};

use crate::layout::style;
use crate::ui::UI_STRINGS;

pub fn styled_list<'a>(
    items: Vec<ListItem<'a>>,
    block: Block<'a>,
    highlight_style: ratatui::style::Style,
) -> List<'a> {
    List::new(items)
        .block(block)
        .style(style::text_style())
        .highlight_style(highlight_style)
        .highlight_symbol(UI_STRINGS.list_symbol)
        .highlight_spacing(HighlightSpacing::Always)
}

pub fn draw_list_panel(
    f: &mut ratatui::Frame,
    items: Vec<ListItem>,
    block: Block,
    highlight_style: ratatui::style::Style,
    list_state: &mut ratatui::widgets::ListState,
    area: Rect,
) {
    f.render_stateful_widget(styled_list(items, block, highlight_style), area, list_state);
}
