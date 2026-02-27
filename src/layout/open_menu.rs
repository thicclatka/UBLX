//! Open menu popup (Shift+O): Open (Terminal) / Open (GUI), drawn below the selected row.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::{style, themes};

const OPEN_MENU_WIDTH: u16 = 24;
const OPEN_MENU_HEIGHT: u16 = 4;

const OPEN_TERMINAL: &str = "Open (Terminal)";
const OPEN_GUI: &str = "Open (GUI)";

/// Draw the open menu popup below the selected row in the middle pane. Clamps position so it stays inside the middle area.
pub fn render_open_menu(
    f: &mut Frame,
    selected_index: usize,
    middle_area: Rect,
    content_selected_index: usize,
) {
    let content_top = middle_area.y + 2;
    let mut y = content_top + content_selected_index as u16;
    if y + OPEN_MENU_HEIGHT > middle_area.y + middle_area.height {
        y = middle_area.y + middle_area.height.saturating_sub(OPEN_MENU_HEIGHT);
    }
    let x = middle_area.x + 1;
    let w = OPEN_MENU_WIDTH.min(middle_area.width.saturating_sub(2));
    let rect = Rect::new(x, y, w, OPEN_MENU_HEIGHT);
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Open ")
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = block.inner(rect);
    f.render_widget(&block, rect);

    let line0 = if selected_index == 0 {
        Line::from(Span::styled(
            OPEN_TERMINAL,
            Style::default().bg(t.tab_active_bg).fg(t.tab_active_fg),
        ))
    } else {
        Line::from(OPEN_TERMINAL)
    };
    let line1 = if selected_index == 1 {
        Line::from(Span::styled(
            OPEN_GUI,
            Style::default().bg(t.tab_active_bg).fg(t.tab_active_fg),
        ))
    } else {
        Line::from(OPEN_GUI)
    };
    let content_rect = Rect::new(inner.x, inner.y, inner.width, 2);
    f.render_widget(
        Paragraph::new(vec![line0, line1]).style(style::text_style()),
        content_rect,
    );
}
