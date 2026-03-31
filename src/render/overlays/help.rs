//! Help overlay: keybinding tables. Sections depend on the active main tab; the main-tab digit row
//! matches visible tabs ([`crate::ui::main_tab_keys_help_keys_line`]). Lenses omits middle-pane sort (`s`).

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};

use crate::layout::{setup::MainMode, style};
use crate::themes;
use crate::ui::{
    COMMAND_MODE_DESCRIPTIONS, UI_CONSTANTS, UI_GLYPHS, UI_STRINGS, main_tab_keys_help_keys_line,
};

macro_rules! help_entries {
    ($( ($key:expr, $action:expr) ),* $(,)?) => {
        &[ $( ($key, $action) ),* ]
    };
}

const WIDTH_LIMIT: usize = 24;
const DESC_MIN_WIDTH: usize = WIDTH_LIMIT - 4;

/// General: navigation, motion, sort, pane focus, Ctrl+j/k, quit/help.
const HELP_GENERAL_BROWSER: &[(&str, &str)] = help_entries![
    ("~", "Alternate between Main tabs"),
    (
        "/",
        "Fuzzy catalog filter; Enter (apply) · / (re-edit) · Esc (clear)"
    ),
    (
        "Tab",
        "Switch left or middle pane focus (categories ↔ contents)"
    ),
    ("h | l", "Focus on Left or Middle panes"),
    ("j | k", "Move down / up in Left or Middle panes"),
    (
        "gg | G",
        "Go to top / bottom of list (Left or Middle panes)"
    ),
    ("s", "Cycle sort mode (middle pane)"),
    (
        "Ctrl+j/k | Ctrl+↑↓",
        "Jump down / up by 10 in Left or Middle panes"
    ),
    (
        "q | Esc",
        "Quit (Esc also clears search / find when active)"
    ),
    ("?", "Toggle this help"),
];

/// Same as [`HELP_GENERAL_BROWSER`] without `s` — middle pane has no sort on Lenses.
const HELP_GENERAL_BROWSER_NO_SORT: &[(&str, &str)] = help_entries![
    ("~", "Alternate between Main tabs"),
    (
        "/",
        "Fuzzy catalog filter; Enter (apply) · / (re-edit) · Esc (clear)"
    ),
    (
        "Tab",
        "Switch left or middle pane focus (categories ↔ contents)"
    ),
    ("h | l", "Focus on Left or Middle panes"),
    ("j | k", "Move down / up in Left or Middle panes"),
    (
        "gg | G",
        "Go to top / bottom of list (Left or Middle panes)"
    ),
    (
        "Ctrl+j/k | Ctrl+↑↓",
        "Jump down / up by 10 in Left or Middle panes"
    ),
    (
        "q | Esc",
        "Quit (Esc also clears search / find when active)"
    ),
    ("?", "Toggle this help"),
];

/// Viewer pane: right-pane tab keys, Shift shortcuts (preview scroll, search, fullscreen).
const HELP_VIEWER: &[(&str, &str)] = help_entries![
    (
        "v/t/m/w",
        "Focus on Viewer/Templates/Metadata/Writing tab in right pane"
    ),
    ("Shift+Tab", "Cycle right pane tab(s)"),
    ("Shift+↑↓", "Scroll up / down in preview"),
    ("Shift+J | +K", "Scroll down / up in preview"),
    ("Shift+b | Shift+e", "Jump preview to top / bottom"),
    (
        "Shift+S",
        "Viewer literal search; Enter (apply) · Shift+S (re-edit) · n/N (next/prev) · Esc (clear)"
    ),
    ("Shift+F", "Viewer tab: toggle fullscreen"),
];

/// Command Mode (Ctrl+Space) and single-letter follow-ups.
const HELP_COMMAND_MODE: &[(&str, &str)] = help_entries![
    (
        "Ctrl+Space",
        "Command Mode: press a key next, or wait briefly for the command menu"
    ),
    ("Command Mode + d", COMMAND_MODE_DESCRIPTIONS.duplicates),
    ("Command Mode + t", COMMAND_MODE_DESCRIPTIONS.theme),
    ("Command Mode + s", COMMAND_MODE_DESCRIPTIONS.snapshot),
    ("Command Mode + r", COMMAND_MODE_DESCRIPTIONS.reload),
    ("Command Mode + p", COMMAND_MODE_DESCRIPTIONS.project),
];

/// Space menu: matches [`crate::ui::menu::space_menu_item_labels`] / `UI_STRINGS.space`.
const HELP_BROWSER_SPACE: &[(&str, &str)] = help_entries![
    ("Space → o", "Open — Terminal and/or GUI when available"),
    ("Space → f", "Show in folder"),
    (
        "Space → p",
        "Enhance policy — when offered (Directory rows / policy submenu)"
    ),
    (
        "Space → z",
        "Enhance with ZahirScan — when offered for this file"
    ),
    (
        "Space → l",
        "Add to Lens (most tabs) or Remove from Lens (Lenses tab, Contents)"
    ),
    ("Space → c", "Copy Path"),
    (
        "Space → j",
        "Copy Templates — when this entry has Zahir JSON stored"
    ),
    (
        "Space → r",
        "Rename file (Contents) or rename lens (Categories, Lenses tab)"
    ),
    (
        "Space → d",
        "Delete file (Contents) or delete lens (Categories, Lenses tab)"
    ),
];

/// Settings tab: one General table (digit row + nav + other + closing).
const HELP_SETTINGS_GENERAL: &[(&str, &str)] = help_entries![
    ("Tab", "Switch Global vs Local config scope"),
    ("j | k", "Move up / down in the focused list"),
    ("e", "Open active Settings file in $EDITOR"),
    (
        "q | Esc",
        "Quit (Esc also clears search / find when active)"
    ),
    ("?", "Toggle this help"),
];

/// Second column for the main-tab digit row; same on every tab.
const MAIN_TAB_DIGITS_DESC: &str = "Jump to Main Tab number based on what is visible.";

struct HelpSectionSpec {
    title: &'static str,
    rows: &'static [(&'static str, &'static str)],
    include_digit_row: bool,
}

const BROWSER_SECTIONS: &[HelpSectionSpec] = &[
    HelpSectionSpec {
        title: UI_STRINGS.tables.first_title,
        rows: HELP_GENERAL_BROWSER,
        include_digit_row: true,
    },
    HelpSectionSpec {
        title: UI_STRINGS.dialogs.help_section_viewer,
        rows: HELP_VIEWER,
        include_digit_row: false,
    },
    HelpSectionSpec {
        title: UI_STRINGS.dialogs.command_mode_popup,
        rows: HELP_COMMAND_MODE,
        include_digit_row: false,
    },
    HelpSectionSpec {
        title: UI_STRINGS.dialogs.help_section_space,
        rows: HELP_BROWSER_SPACE,
        include_digit_row: false,
    },
];

const LENSES_SECTIONS: &[HelpSectionSpec] = &[
    HelpSectionSpec {
        title: UI_STRINGS.tables.first_title,
        rows: HELP_GENERAL_BROWSER_NO_SORT,
        include_digit_row: true,
    },
    HelpSectionSpec {
        title: UI_STRINGS.dialogs.help_section_viewer,
        rows: HELP_VIEWER,
        include_digit_row: false,
    },
    HelpSectionSpec {
        title: UI_STRINGS.dialogs.command_mode_popup,
        rows: HELP_COMMAND_MODE,
        include_digit_row: false,
    },
    HelpSectionSpec {
        title: UI_STRINGS.dialogs.help_section_space,
        rows: HELP_BROWSER_SPACE,
        include_digit_row: false,
    },
];

const SETTINGS_SECTIONS: &[HelpSectionSpec] = &[HelpSectionSpec {
    title: UI_STRINGS.tables.first_title,
    rows: HELP_SETTINGS_GENERAL,
    include_digit_row: true,
}];

#[must_use]
fn help_sections(mode: MainMode) -> &'static [HelpSectionSpec] {
    match mode {
        MainMode::Settings => SETTINGS_SECTIONS,
        MainMode::Lenses => LENSES_SECTIONS,
        _ => BROWSER_SECTIONS,
    }
}

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

/// Word-wrap line count for sizing the blurb area above the tables.
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

fn help_max_key_width(sections: &[HelpSectionSpec], main_keys: &str) -> u16 {
    let mut m = main_keys.len();
    for s in sections {
        for (k, _) in s.rows {
            m = m.max(k.len());
        }
        if s.include_digit_row {
            m = m.max(main_keys.len());
        }
    }
    m = m.max(UI_STRINGS.dialogs.help_command.len());
    u16::try_from(m.min(WIDTH_LIMIT)).unwrap_or(0)
}

fn help_max_desc_width(sections: &[HelpSectionSpec]) -> usize {
    let mut m = MAIN_TAB_DIGITS_DESC.len();
    for s in sections {
        for (_, d) in s.rows {
            m = m.max(d.len());
        }
    }
    m
}

/// Inner height: top gap, blurb, gap, then each section (optional gap, title, table), then gap + GitHub footer line.
fn help_inner_height(blurb_lines: usize, sections: &[HelpSectionSpec]) -> usize {
    let mut h = 1 + blurb_lines + 1;
    for (i, s) in sections.iter().enumerate() {
        if i > 0 {
            h += 1;
        }
        h += 1;
        let dr = s.rows.len() + usize::from(s.include_digit_row);
        h += 1 + dr;
    }
    h + 2 // gap before footer + clickable GitHub line
}

fn build_help_table(
    rows: &[(&'static str, &'static str)],
    include_digit: bool,
    main_keys: &str,
    key_width: u16,
    text_style: Style,
) -> Table<'static> {
    let header = Row::new(vec![
        UI_STRINGS.dialogs.help_command,
        UI_STRINGS.dialogs.help_action,
    ])
    .style(style::table_header_style())
    .bottom_margin(0);

    let mut data_rows: Vec<Row> = Vec::new();
    let mut i = 0usize;
    if include_digit {
        data_rows.push(
            Row::new(vec![
                Cell::from(main_keys.to_string()),
                Cell::from(MAIN_TAB_DIGITS_DESC),
            ])
            .style(style::table_row_style(i)),
        );
        i += 1;
    }
    for (k, d) in rows {
        data_rows
            .push(Row::new(vec![Cell::from(*k), Cell::from(*d)]).style(style::table_row_style(i)));
        i += 1;
    }

    Table::new(
        data_rows,
        [
            Constraint::Length(key_width),
            Constraint::Min(u16::try_from(DESC_MIN_WIDTH).unwrap_or(0)),
        ],
    )
    .header(header)
    .column_spacing(1)
    .style(text_style)
}

struct HelpPopupLayout {
    popup_rect: Rect,
    block: Block<'static>,
    chunks: Vec<Rect>,
    main_keys: String,
    tab_blurb_indented: String,
    sections: &'static [HelpSectionSpec],
    key_width: u16,
    text_style: Style,
}

fn compute_help_popup_layout(
    area: Rect,
    main_mode: MainMode,
    has_lenses: bool,
    has_duplicates: bool,
) -> HelpPopupLayout {
    let main_keys = main_tab_keys_help_keys_line(has_lenses, has_duplicates);
    let tab_blurb = help_context_blurb(main_mode);
    let sections = help_sections(main_mode);

    let key_width = help_max_key_width(sections, &main_keys);
    let desc_max = help_max_desc_width(sections);
    let content_w = (key_width as usize + 1 + desc_max).max(48);
    let tab_blurb_indented = format!(" {tab_blurb}");
    let wrap_w = content_w.saturating_sub(6).max(32);
    let blurb_lines = wrap_line_count(&tab_blurb_indented, wrap_w);
    let blurb_h = u16::try_from(blurb_lines).unwrap_or(u16::MAX).max(1);

    let inner_h = help_inner_height(blurb_lines, sections);
    let content_h = inner_h + 1;

    let rect = style::centered_popup_rect(
        area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    );

    let t = themes::current();
    let tab = main_tab_label(main_mode);
    let title_line = Line::from(vec![
        Span::raw(UI_CONSTANTS.empty_space),
        Span::styled(UI_STRINGS.dialogs.help, Style::default().fg(t.text)),
        Span::raw(" — "),
        Span::styled(tab, Style::default().fg(t.hint)),
        Span::raw(UI_CONSTANTS.empty_space),
    ]);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_line.centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let text_style = Style::default().fg(t.text).bg(t.popup_bg);

    let inner = block.inner(rect);

    let mut constraints: Vec<Constraint> = vec![
        Constraint::Length(1),
        Constraint::Length(blurb_h),
        Constraint::Length(1),
    ];
    for (i, s) in sections.iter().enumerate() {
        if i > 0 {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Length(1));
        let dr = s.rows.len() + usize::from(s.include_digit_row);
        constraints.push(Constraint::Length((1 + dr) as u16));
    }
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Length(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner)
        .to_vec();

    HelpPopupLayout {
        popup_rect: rect,
        block,
        chunks,
        main_keys,
        tab_blurb_indented,
        sections,
        key_width,
        text_style,
    }
}

/// Screen rect for the bottom “GitHub” row in the help overlay (for mouse hit-testing).
#[must_use]
pub fn help_github_footer_rect(
    frame_area: Rect,
    main_mode: MainMode,
    has_lenses: bool,
    has_duplicates: bool,
) -> Rect {
    let layout = compute_help_popup_layout(frame_area, main_mode, has_lenses, has_duplicates);
    layout.chunks.last().copied().unwrap_or(Rect::default())
}

pub fn render_help_box(f: &mut Frame, main_mode: MainMode, has_lenses: bool, has_duplicates: bool) {
    let layout = compute_help_popup_layout(f.area(), main_mode, has_lenses, has_duplicates);
    let chunks = &layout.chunks;
    let n = chunks.len();
    debug_assert!(n >= 5, "help popup chunks include footer");

    f.render_widget(Clear, layout.popup_rect);

    let blurb_para = Paragraph::new(layout.tab_blurb_indented.clone())
        .style(layout.text_style)
        .wrap(Wrap { trim: false });
    let top_gap = Paragraph::new("").style(layout.text_style);
    let blurb_table_gap = Paragraph::new("").style(layout.text_style);

    f.render_widget(layout.block, layout.popup_rect);
    f.render_widget(top_gap, chunks[0]);
    f.render_widget(blurb_para, chunks[1]);
    f.render_widget(blurb_table_gap, chunks[2]);

    let mut idx = 3usize;
    for (sec_i, s) in layout.sections.iter().enumerate() {
        if sec_i > 0 {
            idx += 1;
        }
        let title_chunk = chunks[idx];
        idx += 1;
        let table_chunk = chunks[idx];
        idx += 1;

        let title_para = Paragraph::new(Line::from(vec![Span::styled(
            s.title,
            style::table_section_title_style().add_modifier(Modifier::UNDERLINED),
        )]))
        .alignment(Alignment::Center);
        f.render_widget(title_para, title_chunk);

        let table = build_help_table(
            s.rows,
            s.include_digit_row,
            &layout.main_keys,
            layout.key_width,
            layout.text_style,
        );
        let table_rect = style::rect_with_h_pad(table_chunk);
        f.render_widget(table, table_rect);
    }

    debug_assert_eq!(idx, n - 2);

    let t = themes::current();
    let github_line = Line::from(vec![
        Span::styled(format!("{} ", UI_GLYPHS.github_mark), layout.text_style),
        Span::styled(
            "Repo",
            Style::default()
                .fg(t.text)
                .bg(t.popup_bg)
                .add_modifier(Modifier::UNDERLINED)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let footer_gap = Paragraph::new("").style(layout.text_style);
    let footer_para = Paragraph::new(github_line)
        .alignment(Alignment::Center)
        .style(layout.text_style);
    f.render_widget(footer_gap, chunks[n - 2]);
    f.render_widget(footer_para, chunks[n - 1]);
}
