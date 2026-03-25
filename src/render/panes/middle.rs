//! Shared middle-pane drawing: paths list with selection counter at bottom.
//! Used by Delta, Duplicates, and Lenses modes.

use ratatui::Frame;
use ratatui::layout::{HorizontalAlignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use crate::layout::{setup, style};
use crate::render::panes;
use crate::ui::UI_STRINGS;

fn usize_digit_width(n: usize) -> usize {
    if n == 0 { 1 } else { n.ilog10() as usize + 1 }
}

/// Format `current/total` for the middle-pane counter; field width grows with the larger value.
#[must_use]
pub fn format_selection_counter(current: usize, total: usize) -> String {
    let w = usize_digit_width(current)
        .max(usize_digit_width(total))
        .max(1);
    format!("{current:>w$}/{total:>w$}")
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
    let current = selected_index.map_or(0, |i| i + 1);
    let total = content_len;
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
    let mid_title = panes::set_title(UI_STRINGS.paths.paths, content_focused);
    let mid_block = panes::panel_block(mid_title, content_focused).title_bottom(
        panes::middle::line_for(state.panels.content_state.selected(), view.content_len),
    );
    let mid_items: Vec<ListItem> = if view.content_len == 0 {
        vec![ListItem::new(if state.search.query.is_empty() {
            UI_STRINGS.list.no_contents
        } else {
            UI_STRINGS.list.no_matches
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
