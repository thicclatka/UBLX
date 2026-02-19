//! Delta mode: left (Added / Mod / Removed), middle (paths filtered by search), right (overview).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};

use super::panels;
use crate::layout::{setup, style};
use crate::ui::UI_STRINGS;

pub(super) fn draw_delta_placeholder(f: &mut Frame, left: Rect, middle: Rect, right: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Delta ");
    f.render_widget(
        Paragraph::new("Loading…")
            .style(style::text_style())
            .block(block),
        left,
    );
    f.render_widget(
        Paragraph::new("—")
            .style(style::text_style())
            .block(Block::default().borders(Borders::ALL)),
        middle,
    );
    f.render_widget(
        Paragraph::new("—").style(style::text_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(UI_STRINGS.delta_right_title),
        ),
        right,
    );
}

/// Parameters for [draw_delta_panes] (avoids too many arguments).
pub(super) struct DrawDeltaPanesParams<'a> {
    pub state: &'a mut setup::UblxState,
    pub delta: &'a setup::DeltaViewData,
    pub view: &'a setup::ViewData,
    pub left: Rect,
    pub middle: Rect,
    pub right: Rect,
}

/// Left-pane labels for Delta: Added / Mod / Removed, with styles.
fn delta_left_labels() -> [(&'static str, Style); 3] {
    [
        (UI_STRINGS.delta_added, style::delta_added()),
        (UI_STRINGS.delta_mod, style::delta_mod()),
        (UI_STRINGS.delta_removed, style::delta_removed()),
    ]
}

pub(super) fn draw_delta_panes(f: &mut Frame, params: DrawDeltaPanesParams<'_>) {
    let state = params.state;
    let focused = matches!(state.focus, setup::PanelFocus::Categories);
    let labels = delta_left_labels();
    let items: Vec<ListItem> = params
        .view
        .filtered_categories
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = labels
                .get(i)
                .map(|(_, st)| *st)
                .unwrap_or(style::text_style());
            let span = ratatui::text::Span::styled(s.as_str(), style);
            ListItem::new(Line::from(span))
        })
        .collect();
    let title = panels::set_title("Delta type", focused);
    let left_block = panels::panel_block(title, focused);
    f.render_stateful_widget(
        panels::styled_list(items, left_block, focused, state.highlight_style),
        params.left,
        &mut state.category_state,
    );

    let content_focused = matches!(state.focus, setup::PanelFocus::Contents);
    let mid_title = panels::set_title("Paths", content_focused);
    let mid_items: Vec<ListItem> = if params.view.content_len == 0 {
        vec![ListItem::new(if state.search_query.is_empty() {
            UI_STRINGS.no_contents
        } else {
            UI_STRINGS.no_matches
        })]
    } else {
        params
            .view
            .iter_contents(None)
            .map(|(path, _, _)| ListItem::new(path.as_str()))
            .collect()
    };
    panels::draw_list_panel(
        f,
        mid_items,
        panels::panel_block(mid_title, content_focused),
        content_focused,
        state.highlight_style,
        &mut state.content_state,
        params.middle,
    );

    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(UI_STRINGS.delta_right_title);
    f.render_widget(&right_block, params.right);
    let right_inner = right_block.inner(params.right);
    f.render_widget(
        Paragraph::new(ratatui::text::Text::from(
            params.delta.overview_text.as_str(),
        ))
        .style(style::text_style())
        .scroll((state.preview_scroll, 0)),
        right_inner,
    );
}
