//! Help overlay: keybinding table; rows and title depend on the active main tab.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};

use crate::layout::setup::MainMode;
use crate::layout::style;
use crate::themes;
use crate::ui::UI_CONSTANTS;
use crate::ui::consts::{UI_STRINGS, main_tab_keys_help_keys_line};

macro_rules! help_entries {
    ($( ($key:expr, $action:expr) ),* $(,)?) => {
        &[ $( ($key, $action) ),* ]
    };
}

const WIDTH_LIMIT: usize = 24;
const DESC_MIN_WIDTH: usize = WIDTH_LIMIT - 4;

/// Keys shown on every main tab.
const HELP_ALWAYS: &[(&str, &str)] = help_entries![
    ("Shift+Tab", "Alternate between Main tabs"),
    ("Ctrl+t", "Theme selector — writes to local settings file"),
    ("Ctrl+s", "Take snapshot"),
];

/// Snapshot, Delta, Lenses, Duplicates: three-pane + preview workflows.
const HELP_BROWSER: &[(&str, &str)] = help_entries![
    ("Ctrl+d", "Run duplicate detection, show Duplicates tab"),
    (
        "/",
        "Fuzzy catalog filter; Enter (apply) · / (re-edit) · Esc (clear)"
    ),
    (
        "Ctrl+f",
        "Viewer literal find; Enter (apply) · Ctrl+f (re-edit) · n/N (next/prev) · Esc (clear)"
    ),
    ("s", "Cycle sort mode (middle pane)"),
    (
        "Shift+E",
        "Enhance selected file with ZahirScan (when available)"
    ),
    ("h | l", "Focus on Left or Middle panes"),
    ("j | k", "Move down / up in Left or Middle panes"),
    (
        "Ctrl+j/k | Ctrl+↑↓",
        "Jump down / up by 10 in Left or Middle panes"
    ),
    (
        "gg | G",
        "Go to top / bottom of list (Left or Middle panes)"
    ),
    ("Ctrl+b | +e", "Scroll to beginning / end of right pane"),
    ("Shift+↑↓", "Scroll up / down in preview"),
    ("Shift+J | +K", "Scroll down / up in preview"),
    (
        "Tab",
        "Switch left or middle pane focus (categories ↔ contents)"
    ),
    (
        "v/t/m/w",
        "Focus on Viewer/Templates/Metadata/Writing tab in right pane"
    ),
    ("Ctrl+v", "Cycle right pane tab(s)"),
    ("Shift+O", "Open menu (Terminal/GUI) for selected file"),
    (
        "Ctrl+l",
        "Add to lens: menu (Create New Lens or pick lens), then add current file"
    ),
    (
        "Space",
        "Context menu — letters in (…) match rows; optional rows omit their key"
    ),
];

/// Settings tab: config editor and scope switching.
const HELP_SETTINGS: &[(&str, &str)] = help_entries![
    ("Tab", "Switch Global vs Local config scope"),
    ("Shift+O", "Open active Settings file in $EDITOR"),
    ("j | k", "Move up / down in the focused list"),
];

/// Second column for the main-tab digit row; same on every tab.
const MAIN_TAB_DIGITS_DESC: &str = "Jump to Main Tab number based on what is visible.";

const HELP_CLOSING: &[(&str, &str)] = help_entries![
    (
        "q | Esc",
        "Quit (Esc also clears search / find when active)"
    ),
    ("?", "Toggle this help")
];

#[must_use]
fn main_tab_label(mode: MainMode) -> &'static str {
    match mode {
        MainMode::Snapshot => UI_STRINGS.main_tabs.snapshot,
        MainMode::Delta => UI_STRINGS.main_tabs.delta,
        MainMode::Settings => UI_STRINGS.main_tabs.settings,
        MainMode::Duplicates => UI_STRINGS.main_tabs.duplicates,
        MainMode::Lenses => UI_STRINGS.main_tabs.lenses,
    }
}

#[must_use]
fn help_context_blurb(mode: MainMode) -> String {
    let prefix = "Current tab: ";
    let viewer_tabs = "right pane file viewer.";
    let body = match mode {
        MainMode::Snapshot => {
            format!("category tree, file list, and {viewer_tabs}")
        }
        MainMode::Delta => "snapshot overview and added / modified / removed lists.".to_string(),
        MainMode::Lenses => {
            format!("lens names, paths in the selected lens, and {viewer_tabs}")
        }
        MainMode::Duplicates => {
            format!("duplicate groups, member paths, and {viewer_tabs}")
        }
        MainMode::Settings => "edit Global or Local settings.".to_string(),
    };
    format!("{prefix}{body}")
}

/// Word-wrap line count for sizing the blurb area above the table.
fn wrap_line_count(text: &str, max_width: usize) -> usize {
    if max_width < 8 {
        return 1;
    }
    let mut lines = 0usize;
    let mut line_len = 0usize;
    for word in text.split_whitespace() {
        let w = word.chars().count();
        if line_len == 0 {
            line_len = w;
        } else if line_len + 1 + w <= max_width {
            line_len += 1 + w;
        } else {
            lines += 1;
            line_len = w;
        }
    }
    if line_len > 0 {
        lines += 1;
    }
    lines.max(1)
}

fn collect_help_rows(mode: MainMode) -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&'static str, &'static str)> = Vec::new();
    v.extend_from_slice(HELP_ALWAYS);
    match mode {
        MainMode::Settings => v.extend_from_slice(HELP_SETTINGS),
        _ => v.extend_from_slice(HELP_BROWSER),
    }
    v.extend_from_slice(HELP_CLOSING);
    v
}

pub fn render_help_box(f: &mut Frame, main_mode: MainMode) {
    let main_keys = main_tab_keys_help_keys_line();
    let tab_blurb = help_context_blurb(main_mode);
    let rows = collect_help_rows(main_mode);

    let key_width = u16::try_from(
        rows.iter()
            .map(|(k, _)| k.len())
            .chain(std::iter::once(main_keys.len()))
            .max()
            .unwrap_or(0)
            .min(WIDTH_LIMIT),
    )
    .unwrap_or(0);
    let desc_max = rows
        .iter()
        .map(|(_, d)| d.len())
        .chain(std::iter::once(MAIN_TAB_DIGITS_DESC.len()))
        .max()
        .unwrap_or(0);
    let content_w = (key_width as usize + 1 + desc_max).max(48);
    let tab_blurb_indented = format!(" {tab_blurb}");
    let wrap_w = content_w.saturating_sub(6).max(32);
    let blurb_lines = wrap_line_count(&tab_blurb_indented, wrap_w);
    let data_row_count = 1 + rows.len();
    let table_line_count = 1 + data_row_count;
    let content_h = blurb_lines + table_line_count + 3 + 1 + 1;

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
    let tab = main_tab_label(main_mode);
    let title_line = Line::from(vec![
        Span::styled(UI_STRINGS.dialogs.help, Style::default().fg(t.text)),
        Span::raw(" — "),
        Span::styled(tab, Style::default().fg(t.hint)),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_line.centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let text_style = Style::default().fg(t.text).bg(t.popup_bg);

    let inner = block.inner(rect);
    let blurb_h = u16::try_from(blurb_lines).unwrap_or(u16::MAX).max(1);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(blurb_h),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let header = Row::new(vec![
        UI_STRINGS.dialogs.help_command,
        UI_STRINGS.dialogs.help_action,
    ])
    .style(style::table_header_style())
    .bottom_margin(0);

    let mut data_rows: Vec<Row> = Vec::with_capacity(data_row_count);
    data_rows.push(
        Row::new(vec![
            Cell::from(main_keys.as_str()),
            Cell::from(MAIN_TAB_DIGITS_DESC),
        ])
        .style(style::table_row_style(0)),
    );
    for (i, (k, d)) in rows.iter().enumerate() {
        data_rows.push(
            Row::new(vec![Cell::from(*k), Cell::from(*d)]).style(style::table_row_style(i + 1)),
        );
    }

    let table_rect = style::rect_with_h_pad(chunks[3]);
    let table = Table::new(
        data_rows,
        [
            Constraint::Length(key_width),
            Constraint::Min(u16::try_from(DESC_MIN_WIDTH).unwrap_or(0)),
        ],
    )
    .header(header)
    .column_spacing(1)
    .style(text_style);

    let blurb_para = Paragraph::new(tab_blurb_indented)
        .style(text_style)
        .wrap(Wrap { trim: false });
    let top_gap = Paragraph::new("").style(text_style);
    let blurb_table_gap = Paragraph::new("").style(text_style);

    f.render_widget(block, rect);
    f.render_widget(top_gap, chunks[0]);
    f.render_widget(blurb_para, chunks[1]);
    f.render_widget(blurb_table_gap, chunks[2]);
    f.render_widget(table, table_rect);
}
