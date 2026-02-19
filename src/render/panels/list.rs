//! Styled list panel drawing.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, List, ListItem};

use crate::layout::style;
use crate::ui::UI_STRINGS;

use ratatui::widgets::HighlightSpacing;

pub(crate) fn styled_list<'a>(
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

pub(crate) fn draw_list_panel(
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
