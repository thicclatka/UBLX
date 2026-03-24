//! First-run: choose whether to index with full `ZahirScan` (`enable_enhance_all`).

use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::layout::{style, themes};
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::format::StringObjTraits;

/// Centered overlay; `selected_index` 0 = Yes, 1 = No.
pub fn render_initial_prompt(f: &mut Frame, selected_index: usize) {
    let area = f.area();
    let items = [
        UI_STRINGS.first_run.enhance_yes,
        UI_STRINGS.first_run.enhance_no,
    ];
    let title = UI_STRINGS.first_run.enhance_prompt_title;
    let footnote = UI_STRINGS.first_run.enhance_prompt_footnote;
    let footnote_line_lens = footnote.lines().map(|l| l.chars().count());
    let footnote_h = footnote.lines().count();
    let content_w = 2 + title
        .chars()
        .count()
        .max(items.iter().map(|s| s.chars().count()).max().unwrap_or(0))
        .max(footnote_line_lens.max().unwrap_or(0));
    // Title + gap + Yes/No + gap + footnote lines
    let content_h = 1 + 1 + items.len() + 1 + footnote_h;
    let rect = style::centered_popup_rect(
        area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    );
    f.render_widget(Clear, rect);
    let theme = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(UI_STRINGS.pad(" First run ")).centered())
        .border_style(Style::default().fg(theme.focused_border))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(rect);
    f.render_widget(&block, rect);

    let mut lines: Vec<Line<'_>> = vec![Line::from(Span::styled(
        title,
        Style::default()
            .fg(theme.tab_active_fg)
            .add_modifier(Modifier::BOLD),
    ))];
    lines.push(Line::from(""));
    for (i, label) in items.iter().enumerate() {
        let st = if i == selected_index {
            Style::default()
                .bg(theme.tab_active_bg)
                .fg(theme.tab_active_fg)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::from(Span::styled(*label, st)));
    }
    lines.push(Line::from(""));
    let hint = Style::default().fg(theme.hint);
    for line in footnote.lines() {
        lines.push(Line::from(Span::styled(line, hint)));
    }
    let para = Paragraph::new(Text::from(lines))
        .block(Block::default())
        .wrap(Wrap { trim: true });
    f.render_widget(para, inner);
}
