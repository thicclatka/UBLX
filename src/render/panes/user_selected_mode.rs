//! List-mode panes: left (group/lens names), middle (paths with counter), right (viewer).
//! Shared by Duplicates and Lenses; only the left-panel title differs.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

use crate::layout::setup;
use crate::ui::UI_STRINGS;

/// Draw the three panes for a list mode: left list from `view.filtered_categories`, middle paths with counter, right viewer.
/// `chunks` must have at least 3 elements: [left, middle, right].
fn draw_user_selected_mode_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    chunks: &[Rect],
    left_title_label: &str,
) {
    let left = chunks[0];
    let middle = chunks[1];
    // let right_rect = chunks[2];
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
    let left_title = super::set_title(left_title_label, focused);
    let left_block = super::panel_block(left_title, focused);
    let left_items: Vec<ListItem> = view
        .filtered_categories
        .iter()
        .map(|s| ListItem::new(s.as_str()))
        .collect();
    super::draw_list_panel(
        f,
        left_items,
        left_block,
        state.panels.highlight_style,
        focused,
        &mut state.panels.category_state,
        left,
    );

    super::draw_paths_list_with_counter(f, state, view, None, middle);

    super::draw_right_pane(f, state, right_content, chunks);
}

/// Draw Duplicates mode panes: left = group names, middle = paths in selected group, right = viewer.
pub fn draw_duplicates_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    chunks: &[Rect],
) {
    draw_user_selected_mode_panes(
        f,
        state,
        view,
        right_content,
        chunks,
        UI_STRINGS.paths.duplicate_group,
    );
}

/// Draw Lenses mode panes: left = lens names, middle = paths in selected lens, right = viewer.
pub fn draw_lenses_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    chunks: &[Rect],
) {
    draw_user_selected_mode_panes(
        f,
        state,
        view,
        right_content,
        chunks,
        UI_STRINGS.paths.lens_group,
    );
}
