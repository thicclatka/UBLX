//! Delete lens confirmation popup.

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::ui::UI_STRINGS;

use super::utils::{ListPopupParams, render_list_popup};

pub fn render_delete_confirm(
    f: &mut Frame,
    lens_name: &str,
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let title = format!("{}'{}'? ", UI_STRINGS.lens.delete_confirm_title, lens_name);
    let items = [UI_STRINGS.lens.delete_yes, UI_STRINGS.lens.delete_no];
    render_list_popup(
        f,
        &ListPopupParams {
            title: &title,
            items: &items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 28,
            max_items: None,
        },
    );
}
