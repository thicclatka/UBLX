//! Table styling: header and alternating row stripes.

use ratatui::style::{Modifier, Style};

use crate::layout::themes;
use crate::ui::UI_CONSTANTS;

use super::{CurrentTheme, ThemeStyles};

/// Style for a table header row (text color, popup bg, bold, underlined). Uses current theme.
#[must_use]
pub fn table_header_style() -> Style {
    let t = CurrentTheme::palette();
    Style::default()
        .fg(t.text)
        .bg(t.popup_bg)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::UNDERLINED)
}

/// Style for a table data row at `index`. Even indices use `popup_bg`; odd use a lightened shade for alternating stripes. Uses current theme.
#[must_use]
pub fn table_row_style(index: usize) -> Style {
    let t = CurrentTheme::palette();
    let bg = if index.is_multiple_of(2) {
        t.popup_bg
    } else {
        themes::adjust_surface_rgb(t.popup_bg, UI_CONSTANTS.table_stripe_lighten, t.appearance)
    };
    Style::default().fg(t.text).bg(bg)
}

/// Style for a section title line in tables (e.g. "General", "Sheet Stats")
#[must_use]
pub fn table_section_title_style() -> Style {
    let t = CurrentTheme::palette();
    Style::default().fg(t.tab_active_fg)
}

/// Style for a sub-section title (e.g. "departments · Columns" under "departments") so it reads as belonging to the previous section.
#[must_use]
pub fn table_section_subtitle_style() -> Style {
    let t = CurrentTheme::palette();
    Style::default().fg(t.hint)
}
