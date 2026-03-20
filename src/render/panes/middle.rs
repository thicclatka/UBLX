//! Shared middle-pane drawing: paths list with selection counter at bottom.
//! Used by Delta, Duplicates, and Lenses modes.

use ratatui::Frame;
use ratatui::layout::{HorizontalAlignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use crate::layout::{setup, style};
use crate::render::panes;
use crate::ui::UI_STRINGS;

/// Max value shown in the counter (e.g. 99999/99999).
pub const MAX_SELECTION_INDEX: usize = 99_999;

/// Format "current/total" for the middle-pane counter (right-aligned, fixed width).
#[must_use]
pub fn format_selection_counter(current: usize, total: usize) -> String {
    format!(
        "{:>5}/{:>5}",
        current.min(MAX_SELECTION_INDEX),
        total.min(MAX_SELECTION_INDEX)
    )
}

/// Styled bottom line for the middle panel: current/total, right-aligned.
#[must_use]
pub fn counter_line(current: usize, total: usize) -> Line<'static> {
    style::node_line(
        &format_selection_counter(current, total),
        HorizontalAlignment::Right,
    )
}

/// Bottom line for the middle panel from list state: pass selected index (from `content_state.selected()`) and content length.
#[must_use]
pub fn line_for(selected_index: Option<usize>, content_len: usize) -> Line<'static> {
    let current = selected_index.map_or(0, |i| i + 1).min(MAX_SELECTION_INDEX);
    let total = content_len.min(MAX_SELECTION_INDEX);
    counter_line(current, total)
}

/// Draw the middle panel: list of paths (from view) with current/total counter at bottom.
/// Empty state shows "No contents" or "No matches" depending on search.
pub fn draw_paths_list_with_counter(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    area: Rect,
) {
    let content_focused = matches!(state.panels.focus, setup::PanelFocus::Contents);
    let mid_title = panes::set_title(UI_STRINGS.paths_label, content_focused);
    let mid_block = panes::panel_block(mid_title, content_focused).title_bottom(
        panes::middle::line_for(state.panels.content_state.selected(), view.content_len),
    );
    let mid_items: Vec<ListItem> = if view.content_len == 0 {
        vec![ListItem::new(if state.search.query.is_empty() {
            UI_STRINGS.no_contents
        } else {
            UI_STRINGS.no_matches
        })]
    } else {
        view.iter_contents(None)
            .map(|(path, _, _)| ListItem::new(path.as_str()))
            .collect()
    };
    panes::draw_list_panel(
        f,
        mid_items,
        mid_block,
        state.panels.highlight_style,
        &mut state.panels.content_state,
        area,
    );
}
