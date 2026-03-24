//! Subtree `[[enhance_policy]]` chooser; line labels come from `UI_STRINGS.space` (auto vs manual batch Zahir).

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::ui::UI_STRINGS;

use super::utils::{ListPopupParams, render_list_popup};

pub fn render_enhance_policy_menu(
    f: &mut Frame,
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let items = &[
        UI_STRINGS.space.enhance_policy_always,
        UI_STRINGS.space.enhance_policy_never,
    ];
    render_list_popup(
        f,
        &ListPopupParams {
            title: " Enhance policy ",
            items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 30,
            max_items: None,
        },
    );
}
