//! Layout splits for panel areas.

use ratatui::layout::{Constraint, Rect};

use crate::layout::style;

/// Split content area into main area and one status line (Latest Snapshot + Search:).
pub(crate) fn split_main_and_status(content_area: Rect) -> (Rect, Rect) {
    let vertical =
        style::split_vertical(content_area, &[Constraint::Min(1), Constraint::Length(1)]);
    (vertical[0], vertical[1])
}
