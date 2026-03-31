//! List popups: open, lens, space/context, enhance policy, lens name/rename prompts.

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table};

use super::utils::{ListPopupParams, POPUP_MENU, render_list_popup, render_text_input_popup};

use crate::layout::{
    setup::{MainMode, SpaceMenuKind},
    style,
};
use crate::themes;
use crate::ui::{CTRL_MENU_ROWS, UI_CONSTANTS, UI_STRINGS, space_menu_item_labels};
use crate::utils::StringObjTraits;

pub fn render_context_menu(
    f: &mut Frame,
    selected_index: usize,
    kind: &SpaceMenuKind,
    main_mode: MainMode,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let title = match kind {
        SpaceMenuKind::FileActions { .. } => " Actions ",
        SpaceMenuKind::LensPanelActions { .. } => " Lens ",
    };
    let labeled = space_menu_item_labels(kind, main_mode);
    let item_refs: Vec<&str> = labeled.iter().map(String::as_str).collect();
    let max_width = labeled
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2)
        .clamp(28, 52) as u16;
    render_list_popup(
        f,
        &ListPopupParams {
            title,
            items: &item_refs,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width,
            max_items: None,
        },
    );
}

pub fn render_open_menu(
    f: &mut Frame,
    selected_index: usize,
    can_show_terminal: bool,
    middle_area: Rect,
    content_selected_index: usize,
) {
    let items: &[&str] = if can_show_terminal {
        &[POPUP_MENU.open_terminal, POPUP_MENU.open_gui]
    } else {
        &[POPUP_MENU.open_gui]
    };
    let sel = if can_show_terminal {
        selected_index.min(1)
    } else {
        0
    };
    render_list_popup(
        f,
        &ListPopupParams {
            title: POPUP_MENU.open_title,
            items,
            selected_index: sel,
            anchor_area: middle_area,
            anchor_row_index: content_selected_index,
            max_width: POPUP_MENU.open_width,
            max_items: None,
        },
    );
}

pub fn render_lens_menu(
    f: &mut Frame,
    selected_index: usize,
    middle_area: Rect,
    content_selected_index: usize,
    lens_names: &[String],
) {
    let items: Vec<&str> = std::iter::once(UI_STRINGS.lens.menu_create_new)
        .chain(lens_names.iter().map(String::as_str))
        .collect();
    render_list_popup(
        f,
        &ListPopupParams {
            title: POPUP_MENU.lens_title,
            items: &items,
            selected_index,
            anchor_area: middle_area,
            anchor_row_index: content_selected_index,
            max_width: POPUP_MENU.lens_width,
            max_items: Some(POPUP_MENU.lens_max_items),
        },
    );
}

pub fn render_lens_name_popup(
    f: &mut Frame,
    middle_area: Rect,
    content_selected_index: usize,
    input: &str,
) {
    render_text_input_popup(
        f,
        UI_STRINGS.lens.name_prompt.trim(),
        input,
        middle_area,
        content_selected_index,
        36,
    );
}

pub fn render_lens_name_prompt(f: &mut Frame, area: Rect, input: &str) {
    let line = Line::from(vec![
        Span::styled(UI_STRINGS.lens.name_prompt, style::hint_text()),
        Span::styled(input, style::search_text()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

pub fn render_lens_rename_prompt(f: &mut Frame, area: Rect, input: &str) {
    let line = Line::from(vec![
        Span::styled(UI_STRINGS.lens.rename_prompt, style::hint_text()),
        Span::styled(input, style::search_text()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// File rename: same centered text-input pattern as [`render_lens_name_popup`].
pub fn render_file_rename_popup(
    f: &mut Frame,
    middle_area: Rect,
    content_selected_index: usize,
    input: &str,
) {
    render_text_input_popup(
        f,
        UI_STRINGS.file.rename_prompt.trim(),
        input,
        middle_area,
        content_selected_index,
        40,
    );
}

/// Subtree `[[enhance_policy]]` chooser; line labels come from `UI_STRINGS.space` (auto vs manual batch Zahir).
pub fn render_enhance_policy_menu(
    f: &mut Frame,
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let items = &[
        UI_STRINGS.space.enhance_policy_always,
        UI_STRINGS.space.enhance_policy_never,
    ];
    render_list_popup(
        f,
        &ListPopupParams {
            title: " Enhance policy ",
            items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 30,
            max_items: None,
        },
    );
}

const CMD_MODE_WIDTH_LIMIT: usize = 24;
const CMD_MODE_DESC_MIN_WIDTH: usize = CMD_MODE_WIDTH_LIMIT - 4;

/// Centered Command Mode table (after Ctrl+Space, timeout with no second key). Same header/row styling as help.
pub fn render_ctrl_chord_menu(f: &mut Frame, full_area: Rect) {
    let rows = CTRL_MENU_ROWS;
    let n = rows.len();
    if n == 0 {
        return;
    }

    let t = themes::current();
    let key_width = u16::try_from(
        rows.iter()
            .map(|(k, _)| k.chars().count())
            .chain(std::iter::once(
                UI_STRINGS.dialogs.command_mode_key_column.chars().count(),
            ))
            .max()
            .unwrap_or(0)
            .min(CMD_MODE_WIDTH_LIMIT),
    )
    .unwrap_or(0);

    let desc_max = rows
        .iter()
        .map(|(_, d)| d.chars().count())
        .chain(std::iter::once(
            UI_STRINGS.dialogs.help_action.chars().count(),
        ))
        .max()
        .unwrap_or(0);

    let content_w = (key_width as usize + 1 + desc_max).max(48);
    // Table needs 1 header + n rows inside `Block::inner`. Do not add border slop here — the block
    // already sits inside `centered_popup_rect`; extra `content_h` only makes `inner` taller than
    // the table and leaves empty rows under the last line.
    let content_h = n + 2;

    let rect = style::centered_popup_rect(
        full_area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    );
    f.render_widget(Clear, rect);

    let title = UI_STRINGS.pad(UI_STRINGS.dialogs.command_mode_popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title).centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let text_style = Style::default().fg(t.text).bg(t.popup_bg);

    let inner = block.inner(rect);
    let table_rect = style::rect_with_h_pad(inner);

    let header = Row::new(vec![
        UI_STRINGS.dialogs.command_mode_key_column,
        UI_STRINGS.dialogs.help_action,
    ])
    .style(style::table_header_style())
    .bottom_margin(0);

    let mut data_rows: Vec<Row> = Vec::with_capacity(n);
    for (i, (k, d)) in rows.iter().enumerate() {
        data_rows
            .push(Row::new(vec![Cell::from(*k), Cell::from(*d)]).style(style::table_row_style(i)));
    }

    let table = Table::new(
        data_rows,
        [
            Constraint::Length(key_width),
            Constraint::Min(u16::try_from(CMD_MODE_DESC_MIN_WIDTH).unwrap_or(0)),
        ],
    )
    .header(header)
    .column_spacing(1)
    .style(text_style);

    f.render_widget(&block, rect);
    f.render_widget(table, table_rect);
}

/// Centered table of indexed root paths (Command Mode + `p`). Same block + table styling as [`render_ctrl_chord_menu`].
pub fn render_ublx_switch_picker(
    f: &mut Frame,
    full_area: Rect,
    sw: &crate::layout::setup::UblxSwitchPickerState,
) {
    let t = themes::current();
    let title = UI_STRINGS.pad(UI_STRINGS.dialogs.ublx_switch_popup);
    let n = sw.roots.len();
    let path_width = if n == 0 {
        UI_STRINGS.dialogs.ublx_switch_empty.chars().count()
    } else {
        sw.roots
            .iter()
            .map(|p| p.to_string_lossy().chars().count())
            .max()
            .unwrap_or(0)
    }
    .max(UI_STRINGS.dialogs.ublx_switch_column_path.chars().count())
    .max(48);

    let content_w = path_width.max(48);
    let content_h = if n == 0 { 3 } else { n + 2 };

    let rect = style::centered_popup_rect(
        full_area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    );
    f.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(title).centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let text_style = Style::default().fg(t.text).bg(t.popup_bg);

    let inner = block.inner(rect);
    let table_rect = style::rect_with_h_pad(inner);

    let header = Row::new(vec![UI_STRINGS.dialogs.ublx_switch_column_path])
        .style(style::table_header_style())
        .bottom_margin(0);

    let mut data_rows: Vec<Row> = Vec::with_capacity(n.max(1));
    if n == 0 {
        data_rows.push(
            Row::new(vec![Cell::from(UI_STRINGS.dialogs.ublx_switch_empty)])
                .style(style::table_row_style(0)),
        );
    } else {
        for (i, path) in sw.roots.iter().enumerate() {
            let label = path.display().to_string();
            let row_style = if i == sw.selected_index {
                Style::default().bg(t.tab_active_bg).fg(t.tab_active_fg)
            } else {
                style::table_row_style(i)
            };
            data_rows.push(Row::new(vec![Cell::from(label)]).style(row_style));
        }
    }

    let path_col_w = u16::try_from(path_width.min(200)).unwrap_or(200);
    let table = Table::new(data_rows, [Constraint::Min(path_col_w)])
        .header(header)
        .column_spacing(0)
        .style(text_style);

    f.render_widget(&block, rect);
    f.render_widget(table, table_rect);
}
