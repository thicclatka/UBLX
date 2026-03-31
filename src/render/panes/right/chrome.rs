//! Title, tab list, bordered block, and footer lines for the right pane.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::layout::{
    setup::{RightPaneContent, RightPaneMode, UblxState, ViewData},
    style,
};
use crate::render::{panes, viewers::image as viewer_image};
use crate::themes;
use crate::ui::{UI_STRINGS, chord_chrome_active};
use crate::utils::{StringObjTraits, format_bytes};

use super::content::{haystack_for_right_pane_mode, literal_match_count};

pub fn title(state: &UblxState) -> String {
    let label = match state.right_pane_mode {
        RightPaneMode::Viewer => UI_STRINGS.pane.viewer,
        RightPaneMode::Templates => UI_STRINGS.pane.templates,
        RightPaneMode::Metadata => UI_STRINGS.pane.metadata,
        RightPaneMode::Writing => UI_STRINGS.pane.writing,
    };
    UI_STRINGS.pad(label)
}

/// Horizontal width available inside a bordered right pane (matches [`right_pane_block`] inner).
#[must_use]
pub fn right_pane_inner_content_width(area: Rect) -> u16 {
    Block::default().borders(Borders::ALL).inner(area).width
}

#[must_use]
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

/// PDF / size / mtime footer for the Viewer tab only (right-aligned cluster).
pub fn right_pane_footer_line(
    state: &mut UblxState,
    right_content: &RightPaneContent,
) -> Option<Line<'static>> {
    viewer_image::sync_pdf_selection_state(state, right_content);
    let pdf_footer = viewer_image::pdf_page_footer_text(right_content, &state.viewer_image);
    let show_footer = state.right_pane_mode == RightPaneMode::Viewer
        && (right_content.snap_meta.size.is_some()
            || right_content.snap_meta.mtime_ns.is_some()
            || pdf_footer.is_some());
    let size_str = right_content.snap_meta.size.map(format_bytes);
    show_footer
        .then(|| {
            style::viewer_footer_line(
                size_str.as_deref(),
                right_content.snap_meta.mtime_ns,
                pdf_footer.as_deref(),
                chord_chrome_active(&state.chrome),
            )
        })
        .flatten()
}

/// Find in `title_bottom`: same span strip as catalog search ([`crate::layout::style::popup_input_line_spans`]) plus optional match count.
/// One line only — full bordered popup height does not fit `title_bottom`.
fn find_title_bottom_spans(state: &UblxState) -> Option<Vec<Span<'static>>> {
    let vf = &state.viewer_find;
    if !vf.title_bottom_visible() {
        return None;
    }
    let submitted = vf.committed && !vf.active;
    let mut spans = style::popup_input_line_spans(
        UI_STRINGS.search.find_label.to_string(),
        vf.query.clone(),
        submitted,
    );
    let total = vf.ranges.len();
    if total > 0 {
        let bg = themes::current().popup_bg;
        let count_fg = style::popup_input_accent_color(submitted);
        let cur = vf.current + 1;
        spans.push(Span::styled("  ", Style::default().bg(bg)));
        spans.push(Span::styled(
            format!("{cur}/{total}"),
            Style::default().fg(count_fg).bg(bg),
        ));
    }
    Some(spans)
}

fn combine_spans_left_right(
    left: Vec<Span<'static>>,
    right: Vec<Span<'static>>,
    inner_width: u16,
) -> Line<'static> {
    let lw = Line::from(left.clone()).width();
    let rw = Line::from(right.clone()).width();
    let iw = inner_width as usize;
    let gap = iw
        .saturating_sub(lw + rw)
        .max(usize::from(lw > 0 && rw > 0));
    let mut spans = left;
    spans.push(Span::raw(" ".repeat(gap)));
    spans.extend(right);
    Line::from(spans)
}

/// Bottom `title_bottom` line: popup-styled find (left) + viewer meta (right), same placement as before.
#[must_use]
pub fn right_pane_bottom_line(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    inner_width: u16,
) -> Option<Line<'static>> {
    let find_spans = find_title_bottom_spans(state);
    let viewer_line = right_pane_footer_line(state, right_content);
    match (find_spans, viewer_line) {
        (None, None) => None,
        (Some(l), None) => Some(Line::from(l)),
        (None, Some(line)) => Some(line),
        (Some(l), Some(line)) => Some(combine_spans_left_right(l, line.spans, inner_width)),
    }
}

/// Tab row with optional per-tab literal match counts (`Label ·n`) when find is active.
#[must_use]
pub fn right_pane_tab_spans(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    tabs: &[(RightPaneMode, &'static str)],
    text_w: u16,
) -> Vec<Span<'static>> {
    let show_counts = state.viewer_find.find_affects_view();
    let mut out: Vec<Span<'static>> = Vec::new();
    for (mode, label) in tabs {
        let label_s = if show_counts {
            let hay = haystack_for_right_pane_mode(state, right_content, text_w, *mode);
            let n = literal_match_count(&hay, state.viewer_find.query.trim());
            if n > 0 {
                format!("{label} ·{n}")
            } else {
                (*label).to_string()
            }
        } else {
            (*label).to_string()
        };
        out.extend(style::tab_node_segment(
            label_s.as_str(),
            *mode == state.right_pane_mode,
            chord_chrome_active(&state.chrome),
        ));
    }
    out
}

pub fn right_pane_footer_line_fullscreen(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    view: &ViewData,
    inner_width: u16,
) -> Line<'static> {
    let find_spans = find_title_bottom_spans(state);
    let mut right_spans = panes::middle::line_for(
        state.panels.content_state.selected(),
        view.content_len,
        state.main_mode,
        state.panels.content_sort,
        chord_chrome_active(&state.chrome),
    )
    .spans;
    if let Some(viewer_line) = right_pane_footer_line(state, right_content) {
        right_spans.extend(viewer_line.spans);
    }
    match find_spans {
        None => Line::from(right_spans).right_aligned(),
        Some(left) if right_spans.is_empty() => Line::from(left),
        Some(left) => combine_spans_left_right(left, right_spans, inner_width),
    }
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
