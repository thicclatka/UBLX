//! Shared list popup and text-input popup drawing.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::{style, themes};

pub const DEFAULT_MAX_ITEMS: usize = 20;

pub struct PopupMenuConfig {
    pub open_title: &'static str,
    pub open_width: u16,
    pub open_terminal: &'static str,
    pub open_gui: &'static str,
    pub lens_title: &'static str,
    pub lens_width: u16,
    pub lens_max_items: usize,
}

pub const POPUP_MENU: PopupMenuConfig = PopupMenuConfig {
    open_title: " Open ",
    open_width: 24,
    open_terminal: "Open (Terminal)",
    open_gui: "Open (GUI)",
    lens_title: " Add to lens ",
    lens_width: 28,
    lens_max_items: 12,
};

pub struct ListPopupParams<'a> {
    pub title: &'a str,
    pub items: &'a [&'a str],
    pub selected_index: usize,
    pub anchor_area: Rect,
    pub anchor_row_index: usize,
    pub max_width: u16,
    pub max_items: Option<usize>,
}

pub fn render_list_popup(f: &mut Frame, p: ListPopupParams<'_>) {
    let item_count = p.items.len();
    let height_limit = p.max_items.unwrap_or(DEFAULT_MAX_ITEMS);
    let height = (2 + item_count).min(height_limit + 2) as u16;
    let content_top = p.anchor_area.y + 2;
    let mut y = content_top + p.anchor_row_index as u16;
    if y + height > p.anchor_area.y + p.anchor_area.height {
        y = p.anchor_area.y + p.anchor_area.height.saturating_sub(height);
    }
    let x = p.anchor_area.x + 1;
    let w = p.max_width.min(p.anchor_area.width.saturating_sub(2));
    let rect = Rect::new(x, y, w, height);
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(p.title)
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = block.inner(rect);
    f.render_widget(&block, rect);

    let sel_style = Style::default().bg(t.tab_active_bg).fg(t.tab_active_fg);
    let lines: Vec<Line<'_>> = p
        .items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == p.selected_index {
                Line::from(Span::styled(*label, sel_style))
            } else {
                Line::from(*label)
            }
        })
        .collect();
    let content_height = (item_count as u16).min(inner.height);
    let content_rect = Rect::new(inner.x, inner.y, inner.width, content_height);
    f.render_widget(
        Paragraph::new(lines).style(style::text_style()),
        content_rect,
    );
}

pub fn render_text_input_popup(
    f: &mut Frame,
    title: &str,
    content: &str,
    anchor_area: Rect,
    anchor_row_index: usize,
    max_width: u16,
) {
    const HEIGHT: u16 = 3;
    let content_top = anchor_area.y + 2;
    let mut y = content_top + anchor_row_index as u16;
    if y + HEIGHT > anchor_area.y + anchor_area.height {
        y = anchor_area.y + anchor_area.height.saturating_sub(HEIGHT);
    }
    let x = anchor_area.x + 1;
    let w = max_width.min(anchor_area.width.saturating_sub(2));
    let rect = Rect::new(x, y, w, HEIGHT);
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = block.inner(rect);
    f.render_widget(&block, rect);

    let line = Line::from(vec![Span::styled(content, style::search_text())]);
    f.render_widget(Paragraph::new(line).style(style::text_style()), inner);
}
