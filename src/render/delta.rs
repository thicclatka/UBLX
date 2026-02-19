//! Delta mode: left (type list), middle (paths), right (overview text).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};

use super::consts::{panel_title, UiStrings};
use super::panels;
use crate::layout::{setup, style};

const UI: UiStrings = UiStrings::new();

pub(super) fn draw_delta_placeholder(
    f: &mut Frame,
    left: Rect,
    middle: Rect,
    right: Rect,
) {
    let block = Block::default().borders(Borders::ALL).title(" Delta ");
    f.render_widget(
        Paragraph::new("Loading…").style(style::text_style()).block(block),
        left,
    );
    f.render_widget(
        Paragraph::new("—")
            .style(style::text_style())
            .block(Block::default().borders(Borders::ALL)),
        middle,
    );
    f.render_widget(
        Paragraph::new("—")
            .style(style::text_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(UI.delta_right_title),
            ),
        right,
    );
}

pub(super) fn draw_delta_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    delta: &setup::DeltaViewData,
    left: Rect,
    middle: Rect,
    right: Rect,
) {
    let cat_idx = state.category_state.selected().unwrap_or(0).min(2);
    let focused = matches!(state.focus, setup::PanelFocus::Categories);
    let labels: [(&'static str, Style); 3] = [
        (UI.delta_added, style::delta_added()),
        (UI.delta_mod, style::delta_mod()),
        (UI.delta_removed, style::delta_removed()),
    ];
    let items: Vec<ListItem> = labels
        .iter()
        .map(|(label, s)| {
            let span = ratatui::text::Span::styled(*label, *s);
            ListItem::new(Line::from(span))
        })
        .collect();
    let title = panel_title("Delta type", focused);
    let left_block = panels::panel_block(title, focused);
    f.render_stateful_widget(
        panels::styled_list(items, left_block, focused, state.highlight_style),
        left,
        &mut state.category_state,
    );

    let paths = delta.paths_by_index(cat_idx);
    let content_focused = matches!(state.focus, setup::PanelFocus::Contents);
    let mid_title = panel_title("Paths", content_focused);
    let mid_items: Vec<ListItem> = if paths.is_empty() {
        vec![ListItem::new("(none)")]
    } else {
        paths.iter().map(|p| ListItem::new(p.as_str())).collect()
    };
    panels::draw_list_panel(
        f,
        mid_items,
        panels::panel_block(mid_title, content_focused),
        content_focused,
        state.highlight_style,
        &mut state.content_state,
        middle,
    );

    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(UI.delta_right_title);
    f.render_widget(&right_block, right);
    let right_inner = right_block.inner(right);
    f.render_widget(
        Paragraph::new(ratatui::text::Text::from(delta.overview_text.as_str()))
            .style(style::text_style())
            .scroll((state.preview_scroll, 0)),
        right_inner,
    );
}
