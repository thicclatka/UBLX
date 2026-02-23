//! Duplicates mode: left (one name per duplicate group), middle (paths in group), right (viewer for selected path).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

use super::panels;
use crate::layout::setup;
use crate::ui::UI_STRINGS;

/// Draw Duplicates mode panes: left = group names, middle = paths in selected group, right = viewer (same as Snapshot).
pub fn draw_duplicates_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right: &setup::RightPaneContent,
    left: Rect,
    middle: Rect,
    right_rect: Rect,
) {
    let focused = matches!(state.focus, setup::PanelFocus::Categories);
    let left_title = panels::set_title(UI_STRINGS.duplicates_group_label, focused);
    let left_block = panels::panel_block(left_title, focused);
    let left_items: Vec<ListItem> = view
        .filtered_categories
        .iter()
        .map(|s| ListItem::new(s.as_str()))
        .collect();
    panels::draw_list_panel(
        f,
        left_items,
        left_block,
        state.highlight_style,
        &mut state.category_state,
        left,
    );

    let content_focused = matches!(state.focus, setup::PanelFocus::Contents);
    let mid_title = panels::set_title(UI_STRINGS.paths_label, content_focused);
    let mid_items: Vec<ListItem> = if view.content_len == 0 {
        vec![ListItem::new(if state.search_query.is_empty() {
            UI_STRINGS.no_contents
        } else {
            UI_STRINGS.no_matches
        })]
    } else {
        view.iter_contents(None)
            .map(|(path, _, _)| ListItem::new(path.as_str()))
            .collect()
    };
    panels::draw_list_panel(
        f,
        mid_items,
        panels::panel_block(mid_title, content_focused),
        state.highlight_style,
        &mut state.content_state,
        middle,
    );

    panels::draw_right_pane(f, state, right, right_rect);
}
