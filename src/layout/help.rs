use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Clear, Row, Table};

use crate::layout::{style, themes};

/// Build a static slice of (shortcut, action) pairs. Add lines like:
/// `help_entries![ ("keys", "description"), ... ]`
macro_rules! help_entries {
    ($( ($key:expr, $action:expr) ),* $(,)?) => {
        &[ $( ($key, $action) ),* ]
    };
}

/// All help rows: (shortcut, action). Edit this list to change the help popup.
const HELP_ENTRIES: &[(&str, &str)] = help_entries![
    ("1 / 2", "main tabs: Snapshot / Delta"),
    ("Shift+Tab", "alternate Snapshot ↔ Delta"),
    ("/", "search (strict substring)"),
    (
        "Enter",
        "hide search bar (filter stays); Esc to clear search"
    ),
    (
        "Shift+S",
        "take snapshot (runs in background; bumper when done)"
    ),
    ("q / Esc", "quit"),
    ("h / l", "focus Categories / Contents"),
    ("j / k", "move down / up in list"),
    (
        "gg / G",
        "go to top / bottom of list (Categories or Contents)"
    ),
    (
        "Ctrl+b / Ctrl+e",
        "viewer: scroll to beginning / end of preview"
    ),
    (
        "Shift+↑↓",
        "scroll right pane; or double-tap Shift+J / Shift+K to scroll down / up"
    ),
    ("Tab", "switch focus"),
    (
        "t / v / m / w",
        "right pane: Templates / Viewer / Metadata / Writing (m,w only if data exists)"
    ),
    ("Shift+V", "cycle right pane tab (only tabs with data)"),
    (
        "Shift+T",
        "theme selector (j/k preview, Enter save to .ublx.toml, Esc revert)"
    ),
    ("?", "show this help"),
];

pub fn render_help_box(f: &mut Frame) {
    let key_width = HELP_ENTRIES
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(0)
        .min(24) as u16;
    let desc_max = HELP_ENTRIES.iter().map(|(_, d)| d.len()).max().unwrap_or(0);
    let content_w = key_width as usize + 1 + desc_max;
    let content_h = 1 + HELP_ENTRIES.len();
    let area = f.area();
    let rect = style::centered_popup_rect(area, content_w, content_h, 2, 2);
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(" Help ").centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = Style::default().fg(t.text).bg(t.popup_bg);

    let header = Row::new(vec!["Command", "Action"])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .bottom_margin(0);
    let data_rows: Vec<Row> = HELP_ENTRIES
        .iter()
        .map(|(k, d)| Row::new(vec![Cell::from(*k), Cell::from(*d)]))
        .collect();
    let table = Table::new(
        data_rows,
        [Constraint::Length(key_width), Constraint::Min(20)],
    )
    .header(header)
    .column_spacing(1)
    .block(block)
    .style(inner);

    f.render_widget(table, rect);
}
