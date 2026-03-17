//! Shared list popup drawn below the selected row (Open menu, Lens menu). All menu config and render entry points live here.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::setup::{MainMode, SpaceMenuKind};
use crate::layout::{style, themes};
use crate::ui::UI_STRINGS;

const DEFAULT_MAX_ITEMS: usize = 20;

/// Constants for Open and Lens popup menus (labels, widths, limits).
pub struct PopupMenuConfig {
    pub open_title: &'static str,
    pub open_width: u16,
    pub open_terminal: &'static str,
    pub open_gui: &'static str,
    pub lens_title: &'static str,
    pub lens_width: u16,
    pub lens_max_items: usize,
}

pub const POPUP_MENU: PopupMenuConfig = PopupMenuConfig {
    open_title: " Open ",
    open_width: 24,
    open_terminal: "Open (Terminal)",
    open_gui: "Open (GUI)",
    lens_title: " Add to lens ",
    lens_width: 28,
    lens_max_items: 12,
};

/// Parameters for [render_list_popup] (keeps arg count under clippy limit).
struct ListPopupParams<'a> {
    title: &'a str,
    items: &'a [&'a str],
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
    max_width: u16,
    max_items: Option<usize>,
}

/// Draw a list popup below the selected row in the given area.
fn render_list_popup(f: &mut Frame, p: ListPopupParams<'_>) {
    let item_count = p.items.len();
    let height_limit = p.max_items.unwrap_or(DEFAULT_MAX_ITEMS);
    let height = (2 + item_count).min(height_limit + 2) as u16;
    let content_top = p.anchor_area.y + 2;
    let mut y = content_top + p.anchor_row_index as u16;
    if y + height > p.anchor_area.y + p.anchor_area.height {
        y = p.anchor_area.y + p.anchor_area.height.saturating_sub(height);
    }
    let x = p.anchor_area.x + 1;
    let w = p.max_width.min(p.anchor_area.width.saturating_sub(2));
    let rect = Rect::new(x, y, w, height);
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(p.title)
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = block.inner(rect);
    f.render_widget(&block, rect);

    let sel_style = Style::default().bg(t.tab_active_bg).fg(t.tab_active_fg);
    let lines: Vec<Line<'_>> = p
        .items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == p.selected_index {
                Line::from(Span::styled(*label, sel_style))
            } else {
                Line::from(*label)
            }
        })
        .collect();
    let content_height = (item_count as u16).min(inner.height);
    let content_rect = Rect::new(inner.x, inner.y, inner.width, content_height);
    f.render_widget(
        Paragraph::new(lines).style(style::text_style()),
        content_rect,
    );
}

/// Draw the Open menu (Shift+O or Space → Open…). When `can_show_terminal` is true, show Open (Terminal) and Open (GUI); otherwise only Open (GUI).
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
        ListPopupParams {
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

/// Draw the Lens menu (Shift+L) below the selected row. First item is "Create New Lens", then each lens name.
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
        ListPopupParams {
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

/// Draw the "Lens name: " prompt and current input in the status area (bottom), like the search line.
pub fn render_lens_name_prompt(f: &mut Frame, area: Rect, input: &str) {
    let line = Line::from(vec![
        Span::styled(UI_STRINGS.lens_name_prompt, style::hint_text()),
        Span::styled(input, style::search_text()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// Draw the "Rename lens: " prompt and current input in the status area.
pub fn render_lens_rename_prompt(f: &mut Frame, area: Rect, input: &str) {
    let line = Line::from(vec![
        Span::styled(UI_STRINGS.lens_rename_prompt, style::hint_text()),
        Span::styled(input, style::search_text()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// Draw the spacebar context menu (Open… / Add to Lens… or Remove from Lens; or Rename / Delete when on lens list).
pub fn render_space_menu(
    f: &mut Frame,
    selected_index: usize,
    kind: &SpaceMenuKind,
    main_mode: MainMode,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let (title, items): (&str, Vec<&str>) = match kind {
        SpaceMenuKind::FileActions { .. } => {
            let items: Vec<&str> = if main_mode == MainMode::Lenses {
                vec![
                    UI_STRINGS.space_menu_open,
                    UI_STRINGS.space_menu_remove_from_lens,
                ]
            } else {
                vec![
                    UI_STRINGS.space_menu_open,
                    UI_STRINGS.space_menu_add_to_lens,
                ]
            };
            (" Actions ", items)
        }
        SpaceMenuKind::LensPanelActions { .. } => (
            " Lens ",
            vec![UI_STRINGS.space_menu_rename, UI_STRINGS.space_menu_delete],
        ),
    };
    render_list_popup(
        f,
        ListPopupParams {
            title,
            items: &items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 24,
            max_items: None,
        },
    );
}

/// Draw the "Delete lens 'X'? Yes / No" confirmation popup.
pub fn render_delete_confirm(
    f: &mut Frame,
    lens_name: &str,
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let title = format!("{}'{}'? ", UI_STRINGS.lens_delete_confirm_title, lens_name);
    let items = [UI_STRINGS.lens_delete_yes, UI_STRINGS.lens_delete_no];
    render_list_popup(
        f,
        ListPopupParams {
            title: &title,
            items: &items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 28,
            max_items: None,
        },
    );
}
