//! Delta mode: left (Added / Mod / Removed), middle (paths filtered by search), right (overview).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};

use super::panes;

use crate::layout::{setup, style};
use crate::ui::UI_STRINGS;
use crate::utils::format::StringObjTraits;

pub(super) fn draw_delta_placeholder(f: &mut Frame, left: Rect, middle: Rect, right: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(UI_STRINGS.pad(UI_STRINGS.delta_block_title));
    f.render_widget(
        Paragraph::new(UI_STRINGS.delta_loading)
            .style(style::text_style())
            .block(block),
        left,
    );
    let dash = UI_STRINGS.delta_placeholder_dash;
    f.render_widget(
        Paragraph::new(dash)
            .style(style::text_style())
            .block(Block::default().borders(Borders::ALL)),
        middle,
    );
    f.render_widget(
        Paragraph::new(dash).style(style::text_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(UI_STRINGS.pad(UI_STRINGS.delta_right_title)),
        ),
        right,
    );
}

/// Parameters for [draw_delta_panes] (avoids too many arguments).
pub struct DrawDeltaPanesParams<'a> {
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

pub fn draw_delta_panes(f: &mut Frame, params: DrawDeltaPanesParams<'_>) {
    let state = params.state;
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
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
    let title = panes::set_title(UI_STRINGS.delta_type_label, focused);
    let left_block = panes::panel_block(title, focused);
    f.render_stateful_widget(
        panes::styled_list(items, left_block, state.panels.highlight_style),
        params.left,
        &mut state.panels.category_state,
    );

    panes::draw_paths_list_with_counter(f, state, params.view, params.middle);

    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(UI_STRINGS.pad(UI_STRINGS.delta_right_title));
    f.render_widget(&right_block, params.right);
    let right_inner = right_block.inner(params.right);
    f.render_widget(
        Paragraph::new(ratatui::text::Text::from(
            params.delta.overview_text.as_str(),
        ))
        .style(style::text_style())
        .scroll((state.panels.preview_scroll, 0)),
        right_inner,
    );
}
