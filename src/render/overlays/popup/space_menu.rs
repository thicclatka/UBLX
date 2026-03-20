//! Spacebar context menu.

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::layout::setup::{MainMode, SpaceMenuKind};
use crate::ui::UI_STRINGS;

use super::utils::{ListPopupParams, render_list_popup};

pub fn render_space_menu(
    f: &mut Frame,
    selected_index: usize,
    kind: &SpaceMenuKind,
    main_mode: MainMode,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let (title, items): (&str, Vec<&str>) = match kind {
        SpaceMenuKind::FileActions { .. } => {
            let items: Vec<&str> = if main_mode == MainMode::Lenses {
                vec![
                    UI_STRINGS.space_menu_open,
                    UI_STRINGS.space_menu_remove_from_lens,
                ]
            } else {
                vec![
                    UI_STRINGS.space_menu_open,
                    UI_STRINGS.space_menu_add_to_lens,
                ]
            };
            (" Actions ", items)
        }
        SpaceMenuKind::LensPanelActions { .. } => (
            " Lens ",
            vec![UI_STRINGS.space_menu_rename, UI_STRINGS.space_menu_delete],
        ),
    };
    render_list_popup(
        f,
        &ListPopupParams {
            title,
            items: &items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 24,
            max_items: None,
        },
    );
}
