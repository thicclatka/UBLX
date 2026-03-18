//! Theme selector overlay: list of theme names; highlight shows theme preview.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::layout::{style, themes};
use crate::ui::{UI_CONSTANTS, UI_GLYPHS, UI_STRINGS};
use crate::utils::format::StringObjTraits;

/// Draw centered popup with theme list; each row shows a small swatch (theme background lightened) then the name.
pub fn render_theme_selector(f: &mut Frame, selected_index: usize) {
    let area = f.area();
    let opts = themes::theme_options();
    let rect = popup_rect(area, opts);
    f.render_widget(Clear, rect);

    let block = theme_selector_block(themes::current());
    let items: Vec<ListItem<'_>> = opts
        .iter()
        .enumerate()
        .map(|(i, opt)| theme_option_row(i, opt, selected_index, themes::current()))
        .collect();

    f.render_widget(List::new(items).block(block), rect);
}

fn popup_rect(area: Rect, opts: &[themes::ThemeOption]) -> Rect {
    let content_w = 2 + opts.iter().map(|o| o.display_name.len()).max().unwrap_or(0);
    let content_h = opts.len();
    style::centered_popup_rect(
        area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    )
}

fn theme_selector_block(t: &themes::Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(Line::from(UI_STRINGS.pad(UI_STRINGS.theme_title)).centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg))
}

fn theme_option_row(
    index: usize,
    opt: &themes::ThemeOption,
    selected_index: usize,
    current_theme: &themes::Theme,
) -> ListItem<'static> {
    let swatch_style = Style::default()
        .fg(themes::lighten_rgb(
            opt.theme.background,
            UI_CONSTANTS.swatch_lighten,
        ))
        .bg(themes::lighten_rgb(
            opt.theme.background,
            UI_CONSTANTS.swatch_lighten,
        ));
    let (row_style, pad_style) = row_styles(index, opt, selected_index, current_theme);
    let line = Line::from(vec![
        UI_CONSTANTS.get_empty_span(pad_style),
        Span::styled(UI_GLYPHS.swatch_block.to_string(), swatch_style),
        UI_CONSTANTS.get_empty_span(pad_style),
        Span::styled(opt.display_name, row_style),
    ]);
    ListItem::new(line)
}

fn row_styles(
    index: usize,
    opt: &themes::ThemeOption,
    selected_index: usize,
    current_theme: &themes::Theme,
) -> (Style, Style) {
    if index == selected_index {
        let bg = opt.theme.node_bg;
        (
            Style::default().fg(opt.theme.focused_border).bg(bg),
            Style::default().fg(bg).bg(bg),
        )
    } else {
        let bg = current_theme.popup_bg;
        (
            Style::default().fg(current_theme.text).bg(bg),
            Style::default().fg(bg).bg(bg),
        )
    }
}
