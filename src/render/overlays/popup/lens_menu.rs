//! Lens menu and Create Lens name popup.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::layout::style;
use crate::ui::UI_STRINGS;

use super::utils::{ListPopupParams, POPUP_MENU, render_list_popup, render_text_input_popup};

pub fn render_lens_menu(
    f: &mut Frame,
    selected_index: usize,
    middle_area: Rect,
    content_selected_index: usize,
    lens_names: &[String],
) {
    let items: Vec<&str> = std::iter::once(UI_STRINGS.lens_menu_create_new)
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
        UI_STRINGS.lens_name_prompt.trim(),
        input,
        middle_area,
        content_selected_index,
        36,
    );
}

pub fn render_lens_name_prompt(f: &mut Frame, area: Rect, input: &str) {
    let line = Line::from(vec![
        Span::styled(UI_STRINGS.lens_name_prompt, style::hint_text()),
        Span::styled(input, style::search_text()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

pub fn render_lens_rename_prompt(f: &mut Frame, area: Rect, input: &str) {
    let line = Line::from(vec![
        Span::styled(UI_STRINGS.lens_rename_prompt, style::hint_text()),
        Span::styled(input, style::search_text()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
