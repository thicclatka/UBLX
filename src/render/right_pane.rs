use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use super::consts::UiStrings;
use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::layout::style;

const UI: UiStrings = UiStrings::new();

fn content_str<'a>(state: &'a UblxState, right: &'a RightPaneContent) -> &'a str {
    match state.right_pane_mode {
        RightPaneMode::Templates => right.templates.as_str(),
        RightPaneMode::Metadata => right.metadata.as_deref().unwrap_or(UI.not_available),
        RightPaneMode::Writing => right.writing.as_deref().unwrap_or(UI.not_available),
        RightPaneMode::Viewer => right.viewer.as_deref().unwrap_or(UI.viewer_placeholder),
    }
}

fn title(state: &UblxState) -> &'static str {
    match state.right_pane_mode {
        RightPaneMode::Viewer => UI.viewer,
        RightPaneMode::Templates => UI.templates,
        RightPaneMode::Metadata => UI.metadata,
        RightPaneMode::Writing => UI.writing,
    }
}

fn visible_tabs(right: &RightPaneContent) -> Vec<(RightPaneMode, &'static str)> {
    [
        (RightPaneMode::Templates, UI.tab_templates),
        (RightPaneMode::Viewer, UI.tab_viewer),
        (RightPaneMode::Metadata, UI.tab_metadata),
        (RightPaneMode::Writing, UI.tab_writing),
    ]
    .into_iter()
    .filter(|(mode, _)| match mode {
        RightPaneMode::Templates | RightPaneMode::Viewer => true,
        RightPaneMode::Metadata => right.metadata.is_some(),
        RightPaneMode::Writing => right.writing.is_some(),
    })
    .collect()
}

pub(super) fn draw_right_pane(
    f: &mut Frame,
    state: &UblxState,
    right: &RightPaneContent,
    area: Rect,
) {
    let right_block = Block::default().borders(Borders::ALL).title(title(state));
    let tabs = visible_tabs(right);
    let tab_spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (mode, label))| {
            let s = if *mode == state.right_pane_mode {
                style::tab_active()
            } else {
                style::tab_inactive()
            };
            let sep = if i < tabs.len() - 1 { UI.tab_sep } else { "" };
            vec![Span::styled(*label, s), Span::raw(sep)]
        })
        .collect();
    let right_inner = right_block.inner(area);
    let right_split = style::split_vertical(
        right_inner,
        &[
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ],
    );
    let tab_row_chunks = style::tab_row_padded(right_split[0]);
    let content_chunks = style::tab_row_padded(right_split[2]);

    f.render_widget(&right_block, area);

    f.render_widget(Paragraph::new(Line::from(tab_spans)), tab_row_chunks[1]);

    f.render_widget(
        Paragraph::new(Text::from(content_str(state, right))).scroll((state.preview_scroll, 0)),
        content_chunks[1],
    );
}
