//! Help overlay: keybinding table.

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Clear, Row, Table};

use crate::layout::{style, themes};
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::format::StringObjTraits;

macro_rules! help_entries {
    ($( ($key:expr, $action:expr) ),* $(,)?) => {
        &[ $( ($key, $action) ),* ]
    };
}

const WIDTH_LIMIT: usize = 24;
const DESC_MIN_WIDTH: usize = WIDTH_LIMIT - 4;

const HELP_ENTRIES: &[(&str, &str)] = help_entries![
    (
        "1 | 2 | 3 | 9",
        "Main tabs: Snapshot / Delta / Lenses / Duplicates"
    ),
    ("Shift+Tab", "Alternate between Main tabs"),
    ("Ctrl+d", "Run duplicate detection, show Duplicates tab"),
    ("/", "Search (strict substring search)"),
    ("Shift+S", "Take snapshot"),
    ("h | l", "Focus on Left or Middle panes"),
    ("j | k", "Move down / up in Left or Middle panes"),
    (
        "gg | G",
        "Go to top / bottom of list (Left or Middle panes)"
    ),
    ("Ctrl+b", "Scroll to beginning of preview"),
    ("Ctrl+e", "Scroll to end of preview"),
    ("Shift+↑↓", "Scroll up / down in preview"),
    ("Shift+J | Shift+K", "Scroll down / up in preview"),
    ("Tab", "Switch between left and middle panes"),
    ("v", "Focus on Viewer tab in right pane"),
    ("t", "Focus on Templates tab in right pane (if tab exists)"),
    ("m", "Focus on Metadata tab in right pane (if tab exists)"),
    ("w", "Focus on Writing tab in right pane (if tab exists)"),
    ("Shift+V", "Cycle right pane tab"),
    (
        "Ctrl+t",
        "Theme selector (j/k preview, Enter save to .ublx.toml, Esc cancel)"
    ),
    (
        "Shift+L",
        "Add to lens: menu (Create New Lens or pick lens), then add current file"
    ),
    ("Space", "Open context menu"),
    ("q | Esc", "Quit"),
    ("?", "Show this help"),
];

pub fn render_help_box(f: &mut Frame) {
    let key_width = u16::try_from(
        HELP_ENTRIES
            .iter()
            .map(|(k, _)| k.len())
            .max()
            .unwrap_or(0)
            .min(WIDTH_LIMIT),
    )
    .unwrap_or(0);
    let desc_max = HELP_ENTRIES.iter().map(|(_, d)| d.len()).max().unwrap_or(0);
    let content_w = key_width as usize + 1 + desc_max;
    let content_h = 1 + HELP_ENTRIES.len();
    let area = f.area();
    let rect = style::centered_popup_rect(
        area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    );
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(UI_STRINGS.pad(UI_STRINGS.dialogs.help)).centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = Style::default().fg(t.text).bg(t.popup_bg);

    let header = Row::new(vec![
        UI_STRINGS.dialogs.help_command,
        UI_STRINGS.dialogs.help_action,
    ])
    .style(style::table_header_style())
    .bottom_margin(0);
    let data_rows: Vec<Row> = HELP_ENTRIES
        .iter()
        .enumerate()
        .map(|(i, (k, d))| {
            Row::new(vec![Cell::from(*k), Cell::from(*d)]).style(style::table_row_style(i))
        })
        .collect();
    let table_rect = style::rect_with_h_pad(block.inner(rect));
    let table = Table::new(
        data_rows,
        [
            Constraint::Length(key_width),
            Constraint::Min(u16::try_from(DESC_MIN_WIDTH).unwrap_or(0)),
        ],
    )
    .header(header)
    .column_spacing(1)
    .style(inner);

    f.render_widget(block, rect);
    f.render_widget(table, table_rect);
}
