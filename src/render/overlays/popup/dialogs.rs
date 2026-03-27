use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::utils::{ListPopupParams, render_list_popup};

use crate::layout::style;
use crate::themes;
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::StringObjTraits;

/// Delete lens confirmation popup
pub fn render_delete_confirm(
    f: &mut Frame,
    lens_name: &str,
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let title = format!("{}'{}'? ", UI_STRINGS.lens.delete_confirm_title, lens_name);
    let items = [UI_STRINGS.lens.delete_yes, UI_STRINGS.lens.delete_no];
    render_list_popup(
        f,
        &ListPopupParams {
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

/// First-run single-screen choice: current root + recent prior ublx roots.
pub fn render_startup_root_choice(
    f: &mut Frame,
    selected_index: usize,
    current_path: &Path,
    roots: &[PathBuf],
) {
    let area = f.area();
    let title = UI_STRINGS.first_run.root_choice_title;
    let footnote = UI_STRINGS.first_run.root_choice_footer;
    let current = current_path.to_string_lossy().to_string();
    let current_label = format!("UBLX here: {current}");
    let recent_heading = UI_STRINGS.first_run.recent_heading;
    let footnote_line_lens = footnote.lines().map(|l| l.chars().count());
    let footnote_h = footnote.lines().count();
    let roots_max_w = roots
        .iter()
        .map(|p| p.to_string_lossy().chars().count())
        .max()
        .unwrap_or(0);
    let content_w = 2 + title
        .chars()
        .count()
        .max(current_label.chars().count())
        .max(recent_heading.chars().count())
        .max(roots_max_w)
        .max(footnote_line_lens.max().unwrap_or(0));
    let recent_block_h = if roots.is_empty() { 0 } else { 1 + roots.len() };
    let content_h = 1 + 1 + 1 + 1 + recent_block_h + 1 + footnote_h;
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
        .title(Line::from(UI_STRINGS.pad(UI_STRINGS.first_run.welcome_title)).centered())
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
    let current_style = if selected_index == 0 {
        Style::default()
            .bg(theme.tab_active_bg)
            .fg(theme.tab_active_fg)
    } else {
        Style::default().fg(theme.text)
    };
    lines.push(Line::from(Span::styled(current_label, current_style)));
    if !roots.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            recent_heading,
            Style::default().fg(theme.hint).add_modifier(Modifier::BOLD),
        )));
        for (i, root) in roots.iter().enumerate() {
            let st = if selected_index == i + 1 {
                Style::default()
                    .bg(theme.tab_active_bg)
                    .fg(theme.tab_active_fg)
            } else {
                Style::default().fg(theme.text)
            };
            lines.push(Line::from(Span::styled(
                root.to_string_lossy().to_string(),
                st,
            )));
        }
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

/// Legacy two-step prior-root selector is no longer used.
#[allow(dead_code)]
pub fn render_startup_prior_roots(f: &mut Frame, selected_index: usize, roots: &[PathBuf]) {
    let area = f.area();
    let title = UI_STRINGS.first_run.prior_pick_title;
    let footnote = UI_STRINGS.first_run.path_prompt_footer;
    let footnote_line_lens = footnote.lines().map(|l| l.chars().count());
    let footnote_h = footnote.lines().count();
    let content_w = 2 + title
        .chars()
        .count()
        .max(
            roots
                .iter()
                .map(|p| p.to_string_lossy().chars().count())
                .max()
                .unwrap_or(0),
        )
        .max(footnote_line_lens.max().unwrap_or(0));
    let content_h = 1 + 1 + roots.len() + 1 + footnote_h;
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
        .title(Line::from(UI_STRINGS.pad(" Open prior ublx ")).centered())
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
    for (i, root) in roots.iter().enumerate() {
        let label = root.to_string_lossy().to_string();
        let st = if i == selected_index {
            Style::default()
                .bg(theme.tab_active_bg)
                .fg(theme.tab_active_fg)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::from(Span::styled(label, st)));
    }
    lines.push(Line::from(""));
    for line in footnote.lines() {
        lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(theme.hint),
        )));
    }
    let para = Paragraph::new(Text::from(lines))
        .block(Block::default())
        .wrap(Wrap { trim: true });
    f.render_widget(para, inner);
}
