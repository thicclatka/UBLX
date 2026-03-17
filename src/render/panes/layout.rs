//! Layout splits for panel areas.

use ratatui::layout::Rect;

use crate::layout::style;
use crate::ui::UI_CONSTANTS;

/// Split content area into main area and one status line (Latest Snapshot + Search:).
pub fn split_main_and_status(content_area: Rect) -> (Rect, Rect) {
    let vertical = style::split_vertical(content_area, &UI_CONSTANTS.status_line_constraints());
    (vertical[0], vertical[1])
}
