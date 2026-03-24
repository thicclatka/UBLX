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
        SpaceMenuKind::FileActions {
            show_enhance_directory_policy,
            show_enhance_zahir,
            ..
        } => {
            let mut items = vec![UI_STRINGS.space.open, UI_STRINGS.space.show_in_folder];
            if *show_enhance_directory_policy {
                items.push(UI_STRINGS.space.enhance_policy);
            }
            if *show_enhance_zahir {
                items.push(UI_STRINGS.space.enhance_with_zahirscan);
            }
            if main_mode == MainMode::Lenses {
                items.push(UI_STRINGS.space.remove_from_lens);
            } else {
                items.push(UI_STRINGS.space.add_to_lens);
            }
            (" Actions ", items)
        }
        SpaceMenuKind::LensPanelActions { .. } => (
            " Lens ",
            vec![UI_STRINGS.space.rename, UI_STRINGS.space.delete],
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
            max_width: 34,
            max_items: None,
        },
    );
}
