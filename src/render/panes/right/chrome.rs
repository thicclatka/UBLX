//! Title, tab list, bordered block, and footer lines for the right pane.

use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

use crate::layout::{
    setup::{RightPaneContent, RightPaneMode, UblxState, ViewData},
    style,
};
use crate::render::{panes, viewers::image as viewer_image};
use crate::ui::UI_STRINGS;
use crate::utils::{StringObjTraits, format_bytes};

pub fn title(state: &UblxState) -> String {
    let label = match state.right_pane_mode {
        RightPaneMode::Viewer => UI_STRINGS.pane.viewer,
        RightPaneMode::Templates => UI_STRINGS.pane.templates,
        RightPaneMode::Metadata => UI_STRINGS.pane.metadata,
        RightPaneMode::Writing => UI_STRINGS.pane.writing,
    };
    UI_STRINGS.pad(label)
}

pub fn visible_tabs(right_content: &RightPaneContent) -> Vec<(RightPaneMode, &'static str)> {
    [
        (RightPaneMode::Viewer, UI_STRINGS.pane.tab_viewer),
        (RightPaneMode::Templates, UI_STRINGS.pane.tab_templates),
        (RightPaneMode::Metadata, UI_STRINGS.pane.tab_metadata),
        (RightPaneMode::Writing, UI_STRINGS.pane.tab_writing),
    ]
    .into_iter()
    .filter(|(mode, _)| match mode {
        RightPaneMode::Viewer => true,
        RightPaneMode::Templates => !right_content.templates.is_empty(),
        RightPaneMode::Metadata => right_content.metadata.is_some(),
        RightPaneMode::Writing => right_content.writing.is_some(),
    })
    .collect()
}

pub fn right_pane_footer_line(
    state: &mut UblxState,
    right_content: &RightPaneContent,
) -> Option<Line<'static>> {
    viewer_image::sync_pdf_selection_state(state, right_content);
    let pdf_footer = viewer_image::pdf_page_footer_text(right_content, &state.viewer_image);
    let show_footer = state.right_pane_mode == RightPaneMode::Viewer
        && (right_content.viewer_byte_size.is_some()
            || right_content.viewer_mtime_ns.is_some()
            || pdf_footer.is_some());
    let size_str = right_content.viewer_byte_size.map(format_bytes);
    show_footer
        .then(|| {
            style::viewer_footer_line(
                size_str.as_deref(),
                right_content.viewer_mtime_ns,
                pdf_footer.as_deref(),
            )
        })
        .flatten()
}

pub fn right_pane_footer_line_fullscreen(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    view: &ViewData,
) -> Line<'static> {
    let mut spans = panes::middle::line_for(
        state.panels.content_state.selected(),
        view.content_len,
        state.main_mode,
        state.panels.content_sort,
    )
    .spans;
    if let Some(viewer_line) = right_pane_footer_line(state, right_content) {
        spans.extend(viewer_line.spans);
    }
    Line::from(spans).right_aligned()
}

pub fn right_pane_block<'a>(
    top_title: Option<&'a str>,
    footer_line: Option<&'a Line<'static>>,
) -> Block<'a> {
    let b = Block::default()
        .borders(Borders::ALL)
        .style(style::text_style());
    let b = match top_title {
        Some(t) => b.title(t),
        None => b,
    };
    match footer_line {
        Some(line) => b.title_bottom(line.clone()),
        None => b,
    }
}
