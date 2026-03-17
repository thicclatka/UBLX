//! Block and title helpers for panels.

use ratatui::widgets::{Block, Borders};

use crate::layout::style;

pub fn panel_block<'a, T: Into<ratatui::text::Line<'a>>>(title: T, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title(title)
}

/// Builds a panel block title: `" Label "` or `" ► Label "` when focused.
pub fn set_title(label: &str, focused: bool) -> String {
    if focused {
        format!(" ► {} ", label)
    } else {
        format!(" {} ", label)
    }
}
