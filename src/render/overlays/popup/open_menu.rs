//! Open menu popup (Shift+O or Space → Open…).

use ratatui::Frame;
use ratatui::layout::Rect;

use super::utils::{ListPopupParams, POPUP_MENU, render_list_popup};

pub fn render_open_menu(
    f: &mut Frame,
    selected_index: usize,
    can_show_terminal: bool,
    middle_area: Rect,
    content_selected_index: usize,
) {
    let items: &[&str] = if can_show_terminal {
        &[POPUP_MENU.open_terminal, POPUP_MENU.open_gui]
    } else {
        &[POPUP_MENU.open_gui]
    };
    let sel = if can_show_terminal {
        selected_index.min(1)
    } else {
        0
    };
    render_list_popup(
        f,
        ListPopupParams {
            title: POPUP_MENU.open_title,
            items,
            selected_index: sel,
            anchor_area: middle_area,
            anchor_row_index: content_selected_index,
            max_width: POPUP_MENU.open_width,
            max_items: None,
        },
    );
}
