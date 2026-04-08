//! List-mode panes: left (group/lens names), middle (paths with counter), right (viewer).
//! Shared by Duplicates and Lenses; only the left-panel title differs.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::ListItem;

use crate::layout::setup;
use crate::render::marquee;
use crate::ui::{UI_STRINGS, chord_chrome_active};

/// Draw the three panes for a list mode: left list from `view.filtered_categories`, middle paths with counter, right viewer.
/// `chunks` must have at least 3 elements: [left, middle, right].
fn draw_user_selected_mode_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    chunks: &[Rect],
    left_title_label: &str,
    transparent_page_chrome: bool,
) {
    let left = chunks[0];
    let middle = chunks[1];
    // let right_rect = chunks[2];
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
    let left_title = super::panel_title_line(
        left_title_label,
        focused,
        chord_chrome_active(&state.chrome),
    );
    let left_block = super::panel_block(left_title, focused);
    let max_cols = super::list_row_text_max_cols(left.width);
    let cat_sel = state.panels.category_state.selected().unwrap_or(0);
    let left_items: Vec<ListItem> = view
        .filtered_categories
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let use_marquee = i == cat_sel && focused;
            let text = if use_marquee {
                marquee::visible_line(s.as_str(), max_cols, state.panels.category_marquee.offset)
            } else {
                s.clone()
            };
            ListItem::new(text)
        })
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

    super::draw_paths_list_with_counter(
        f,
        state,
        view,
        None,
        None,
        middle,
        transparent_page_chrome,
    );

    super::draw_right_pane(f, state, right_content, chunks, transparent_page_chrome);
}

/// Draw Duplicates mode panes: left = group names, middle = paths in selected group, right = viewer.
pub fn draw_duplicates_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    chunks: &[Rect],
    transparent_page_chrome: bool,
) {
    draw_user_selected_mode_panes(
        f,
        state,
        view,
        right_content,
        chunks,
        UI_STRINGS.paths.duplicate_group,
        transparent_page_chrome,
    );
}

/// Draw Lenses mode panes: left = lens names, middle = paths in selected lens, right = viewer.
pub fn draw_lenses_panes(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right_content: &setup::RightPaneContent,
    chunks: &[Rect],
    transparent_page_chrome: bool,
) {
    draw_user_selected_mode_panes(
        f,
        state,
        view,
        right_content,
        chunks,
        UI_STRINGS.paths.lens_group,
        transparent_page_chrome,
    );
}
