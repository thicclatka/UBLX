//! Theme selector popup: list of theme names; highlight shows theme preview. Enter to pick and save, Esc to revert.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::layout::{style, themes};
use crate::utils::UI_GLYPHS;

const POPUP_PADDING_W: u16 = 4;
const POPUP_PADDING_H: u16 = 2;
const SWATCH_LIGHTEN: f32 = 0.25;

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

// Centered popup rect for the theme selector: width/height clamped to area, with padding for borders/title.
fn popup_rect(area: Rect, opts: &[themes::ThemeOption]) -> Rect {
    let content_w = 2 + opts.iter().map(|o| o.display_name.len()).max().unwrap_or(0);
    let content_h = opts.len();
    style::centered_popup_rect(area, content_w, content_h, POPUP_PADDING_W, POPUP_PADDING_H)
}

// Block for the theme selector popup: centered title, borders, and background.
fn theme_selector_block(t: &themes::Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(Line::from(" Theme ").centered())
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg))
}

// One row in the theme selector: swatch (theme background lightened) then name.
fn theme_option_row(
    index: usize,
    opt: &themes::ThemeOption,
    selected_index: usize,
    current_theme: &themes::Theme,
) -> ListItem<'static> {
    let swatch_style = Style::default()
        .fg(themes::lighten_rgb(opt.theme.background, SWATCH_LIGHTEN))
        .bg(themes::lighten_rgb(opt.theme.background, SWATCH_LIGHTEN));
    let (row_style, pad_style) = row_styles(index, opt, selected_index, current_theme);
    let line = Line::from(vec![
        Span::styled(UI_GLYPHS.swatch_block.to_string(), swatch_style),
        Span::styled(" ", pad_style),
        Span::styled(opt.display_name, row_style),
    ]);
    ListItem::new(line)
}

// Styles for the theme option row: selected and unselected.
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
