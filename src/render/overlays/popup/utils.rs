//! Shared list popup and text-input popup drawing.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::style;
use crate::themes;

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

pub fn render_list_popup(frame: &mut Frame, params: &ListPopupParams<'_>) {
    let item_count = params.items.len();
    let height_limit = params.max_items.unwrap_or(DEFAULT_MAX_ITEMS);
    let height = (2 + item_count).min(height_limit + 2) as u16;
    let content_top = params.anchor_area.y + 2;
    let mut row_y = content_top + params.anchor_row_index as u16;
    if row_y + height > params.anchor_area.y + params.anchor_area.height {
        row_y = params
            .anchor_area
            .y
            .saturating_add(params.anchor_area.height.saturating_sub(height));
    }
    let col_x = params.anchor_area.x + 1;
    let popup_w = params
        .max_width
        .min(params.anchor_area.width.saturating_sub(2));
    let rect = Rect::new(col_x, row_y, popup_w, height);
    frame.render_widget(Clear, rect);

    let theme = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(params.title)
        .border_style(Style::default().fg(theme.focused_border))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(rect);
    frame.render_widget(&block, rect);

    let sel_style = Style::default()
        .bg(theme.tab_active_bg)
        .fg(theme.tab_active_fg);
    let lines: Vec<Line<'_>> = params
        .items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == params.selected_index {
                Line::from(Span::styled(*label, sel_style))
            } else {
                Line::from(*label)
            }
        })
        .collect();
    let content_height = (item_count as u16).min(inner.height);
    let content_rect = Rect::new(inner.x, inner.y, inner.width, content_height);
    frame.render_widget(
        Paragraph::new(lines).style(style::text_style()),
        content_rect,
    );
}

pub fn render_text_input_popup(
    frame: &mut Frame,
    title: &str,
    content: &str,
    anchor_area: Rect,
    anchor_row_index: usize,
    max_width: u16,
) {
    const HEIGHT: u16 = 3;
    let content_top = anchor_area.y + 2;
    let mut row_y = content_top + anchor_row_index as u16;
    if row_y + HEIGHT > anchor_area.y + anchor_area.height {
        row_y = anchor_area
            .y
            .saturating_add(anchor_area.height.saturating_sub(HEIGHT));
    }
    let col_x = anchor_area.x + 1;
    let popup_w = max_width.min(anchor_area.width.saturating_sub(2));
    let rect = Rect::new(col_x, row_y, popup_w, HEIGHT);
    frame.render_widget(Clear, rect);

    let theme = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme.focused_border))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(rect);
    frame.render_widget(&block, rect);

    let line = Line::from(vec![Span::styled(content, style::search_text())]);
    frame.render_widget(Paragraph::new(line).style(style::text_style()), inner);
}
