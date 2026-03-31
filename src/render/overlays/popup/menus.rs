//! List popups: open, lens, space/context, enhance policy, lens name/rename prompts.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::layout::{
    setup::{MainMode, SpaceMenuKind},
    style,
};
use crate::ui::{UI_STRINGS, menu};

use super::utils::{ListPopupParams, POPUP_MENU, render_list_popup, render_text_input_popup};

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
    let labeled = menu::space_menu_item_labels(kind, main_mode);
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
