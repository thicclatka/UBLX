use std::path::{Path, PathBuf};

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::layout::style;
use crate::themes;
use crate::themes::Palette;
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::StringObjTraits;

use super::utils::{ListPopupParams, render_list_popup};

// --- First-run startup overlays (see [`crate::layout::setup::StartupPromptPhase`]) --------------

/// Clear, size, and draw the standard centered modal frame; returns the inner content [`Rect`].
fn paint_centered_titled_modal(
    f: &mut Frame,
    theme: &Palette,
    area: Rect,
    content_w: usize,
    content_h: usize,
    border_title: &str,
) -> Rect {
    let rect = style::centered_popup_rect(
        area,
        content_w,
        content_h,
        UI_CONSTANTS.popup_padding_w,
        UI_CONSTANTS.popup_padding_h,
    );
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(UI_STRINGS.pad(border_title)).centered())
        .border_style(Style::default().fg(theme.focused_border))
        .style(Style::default().bg(theme.popup_bg));
    let inner = block.inner(rect);
    f.render_widget(&block, rect);
    inner
}

fn lines_selectable_static_labels(
    theme: &Palette,
    items: &[&'static str],
    selected_index: usize,
) -> Vec<Line<'static>> {
    items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let st = if i == selected_index {
                Style::default()
                    .bg(theme.tab_active_bg)
                    .fg(theme.tab_active_fg)
            } else {
                Style::default().fg(theme.text)
            };
            Line::from(Span::styled(*label, st))
        })
        .collect()
}

/// Blank line plus footnote lines in hint style.
fn push_multiline_hint<'a>(lines: &mut Vec<Line<'a>>, theme: &Palette, footnote: &'a str) {
    lines.push(Line::from(""));
    let hint = Style::default().fg(theme.hint);
    for line in footnote.lines() {
        lines.push(Line::from(Span::styled(line, hint)));
    }
}

/// Border title + `items` rows + gap + multi-line hint (used by enhance-all and previous-settings prompts).
fn render_centered_labeled_options_with_footnote(
    f: &mut Frame,
    border_title: &str,
    items: &[&'static str],
    selected_index: usize,
    footnote: &'static str,
) {
    let area = f.area();
    let footnote_line_lens = footnote.lines().map(|l| l.chars().count());
    let footnote_h = footnote.lines().count();
    let content_w = 2 + border_title
        .chars()
        .count()
        .max(items.iter().map(|s| s.chars().count()).max().unwrap_or(0))
        .max(footnote_line_lens.max().unwrap_or(0));
    let content_h = items.len() + 1 + footnote_h;
    let theme = themes::current();
    let inner = paint_centered_titled_modal(f, theme, area, content_w, content_h, border_title);
    let mut lines = lines_selectable_static_labels(theme, items, selected_index);
    push_multiline_hint(&mut lines, theme, footnote);
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default())
            .wrap(Wrap { trim: true }),
        inner,
    );
}

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

/// Delete file or folder under the indexed root (`title` is the full prompt line, e.g. `Delete path'? ` or bulk count).
pub fn render_file_delete_confirm(
    f: &mut Frame,
    title: &str,
    selected_index: usize,
    anchor_area: Rect,
    anchor_row_index: usize,
) {
    let items = [UI_STRINGS.lens.delete_yes, UI_STRINGS.lens.delete_no];
    render_list_popup(
        f,
        &ListPopupParams {
            title,
            items: &items,
            selected_index,
            anchor_area,
            anchor_row_index,
            max_width: 36,
            max_items: None,
        },
    );
}

/// Full-directory `ZahirScan` prompt: whether to turn on `enable_enhance_all` for this root.
///
/// Shown in the startup flow as [`crate::layout::setup::StartupPromptPhase::Enhance`]. `selected_index`:
/// 0 = Yes, 1 = No.
pub fn render_startup_enhance_all_prompt(f: &mut Frame, selected_index: usize) {
    render_centered_labeled_options_with_footnote(
        f,
        UI_STRINGS.first_run.enhance_prompt_title,
        &[
            UI_STRINGS.first_run.enhance_yes,
            UI_STRINGS.first_run.enhance_no,
        ],
        selected_index,
        UI_STRINGS.first_run.enhance_prompt_footnote,
    );
}

/// Prior config for this folder: use cached `ublx.toml` / start clean.
///
/// [`crate::layout::setup::StartupPromptPhase::PreviousSettings`]. `selected_index`: 0 = use saved, 1 = start fresh.
pub fn render_startup_previous_settings_prompt(f: &mut Frame, selected_index: usize) {
    render_centered_labeled_options_with_footnote(
        f,
        UI_STRINGS.first_run.previous_settings_title,
        &[
            UI_STRINGS.first_run.previous_settings_use,
            UI_STRINGS.first_run.previous_settings_fresh,
        ],
        selected_index,
        UI_STRINGS.first_run.previous_settings_footnote,
    );
}

/// Welcome step: confirm the launch directory as the UBLX root or pick a recent indexed path.
///
/// [`crate::layout::setup::StartupPromptPhase::RootChoice`]. `selected_index` 0 = current path; `1..` = `roots[i-1]`.
pub fn render_startup_welcome_root_choice(
    f: &mut Frame,
    selected_index: usize,
    current_path: &Path,
    roots: &[PathBuf],
) {
    let area = f.area();
    let footnote = UI_STRINGS.first_run.root_choice_footer;
    let current = current_path.to_string_lossy().to_string();
    let current_label = format!("{}{}", UI_STRINGS.first_run.ublx_here, current);
    let recent_heading = UI_STRINGS.first_run.recent_heading;
    let footnote_line_lens = footnote.lines().map(|l| l.chars().count());
    let footnote_h = footnote.lines().count();
    let roots_max_w = roots
        .iter()
        .map(|p| p.to_string_lossy().chars().count())
        .max()
        .unwrap_or(0);
    let content_w = 2 + current_label
        .chars()
        .count()
        .max(recent_heading.chars().count())
        .max(roots_max_w)
        .max(footnote_line_lens.max().unwrap_or(0));
    let recent_block_h = if roots.is_empty() { 0 } else { 2 + roots.len() };
    // current row + optional recent block (gap, heading, roots) + gap before footnote + footnote lines
    let content_h = 1 + recent_block_h + 1 + footnote_h;
    let theme = themes::current();
    let inner = paint_centered_titled_modal(
        f,
        theme,
        area,
        content_w,
        content_h,
        UI_STRINGS.first_run.welcome_title,
    );

    let mut lines: Vec<Line<'_>> = Vec::new();
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
    push_multiline_hint(&mut lines, theme, footnote);
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default())
            .wrap(Wrap { trim: true }),
        inner,
    );
}
