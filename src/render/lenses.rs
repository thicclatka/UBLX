//! Lenses mode: left = lens names, middle = paths in selected lens, right = viewer for selected path.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

use super::panes;

use crate::layout::setup;
use crate::ui::UI_STRINGS;

/// Draw Lenses mode panes: left = lens names, middle = paths in selected lens, right = viewer (same as Snapshot).
pub fn draw_lenses_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right: &setup::RightPaneContent,
    left: Rect,
    middle: Rect,
    right_rect: Rect,
) {
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
    let left_title = panes::set_title(UI_STRINGS.lenses_group_label, focused);
    let left_block = panes::panel_block(left_title, focused);
    let left_items: Vec<ListItem> = view
        .filtered_categories
        .iter()
        .map(|s| ListItem::new(s.as_str()))
        .collect();
    panes::draw_list_panel(
        f,
        left_items,
        left_block,
        state.panels.highlight_style,
        &mut state.panels.category_state,
        left,
    );

    panes::draw_paths_list_with_counter(f, state, view, middle);

    panes::draw_right_pane(f, state, right, right_rect);
}
