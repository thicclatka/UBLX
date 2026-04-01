//! Delta mode: left (Added / Mod / Removed), middle (paths filtered by search), right (overview).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, ListItem, Paragraph};

use crate::layout::{setup, style};
use crate::ui::{UI_STRINGS, chord_chrome_active};
use crate::utils::StringObjTraits;

/// Draw placeholder when delta data is missing. `chunks` must have at least 3 elements: [left, middle, right].
pub fn draw_delta_placeholder(f: &mut Frame, chunks: &[Rect]) {
    let left = chunks[0];
    let middle = chunks[1];
    let right = chunks[2];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(UI_STRINGS.pad(UI_STRINGS.delta.left_block_title));
    f.render_widget(
        Paragraph::new(UI_STRINGS.loading.general)
            .style(style::text_style())
            .block(block),
        left,
    );
    let dash = UI_STRINGS.delta.placeholder_dash;
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
                .title(UI_STRINGS.pad(UI_STRINGS.delta.right_title)),
        ),
        right,
    );
}

/// Parameters for [`draw_delta_panes`] (avoids too many arguments).
pub struct DrawDeltaPanesParams<'a> {
    pub state: &'a mut setup::UblxState,
    pub delta: &'a setup::DeltaViewData,
    pub view: &'a setup::ViewData,
    /// Left, middle, right pane rects (at least 3 elements).
    pub chunks: &'a [Rect],
    pub transparent_page_chrome: bool,
}

/// Left-pane labels for Delta: Added / Mod / Removed, with styles.
fn delta_left_labels() -> [(&'static str, Style); 3] {
    [
        (UI_STRINGS.delta.added, style::delta_added()),
        (UI_STRINGS.delta.modified, style::delta_mod()),
        (UI_STRINGS.delta.removed, style::delta_removed()),
    ]
}

pub fn draw_delta_panes(f: &mut Frame, params: DrawDeltaPanesParams<'_>) {
    let state = params.state;
    let left = params.chunks[0];
    let middle = params.chunks[1];
    let right = params.chunks[2];
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
    let labels = delta_left_labels();
    let items: Vec<ListItem> = params
        .view
        .filtered_categories
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = labels.get(i).map_or(style::text_style(), |(_, st)| *st);
            let span = ratatui::text::Span::styled(s.as_str(), style);
            ListItem::new(Line::from(span))
        })
        .collect();
    let title = super::panel_title_line(
        UI_STRINGS.delta.type_label,
        focused,
        chord_chrome_active(&state.chrome),
    );
    let left_block = super::panel_block(title, focused);
    f.render_stateful_widget(
        super::styled_list(items, left_block, state.panels.highlight_style, focused),
        left,
        &mut state.panels.category_state,
    );

    super::draw_paths_list_with_counter(
        f,
        state,
        params.view,
        None,
        None,
        middle,
        params.transparent_page_chrome,
    );

    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(UI_STRINGS.pad(UI_STRINGS.delta.right_title));
    f.render_widget(&right_block, right);
    let right_inner = right_block.inner(right);
    f.render_widget(
        Paragraph::new(ratatui::text::Text::from(
            params.delta.overview_text.as_str(),
        ))
        .style(style::text_style())
        .scroll((state.panels.preview_scroll, 0)),
        right_inner,
    );
}
