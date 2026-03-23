//! Toast overlay: draw a single toast slot (level-colored lines, title, block).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::layout::themes;
use crate::ui::UI_STRINGS;
use crate::utils::format::StringObjTraits;
use crate::utils::notifications::{ToastSlot, level_short, level_style};

/// Draw one toast slot in the given rect (used for stacked toasts).
pub fn render_toast_slot(f: &mut Frame, area: Rect, slot: &ToastSlot) {
    f.render_widget(Clear, area);
    if slot.messages.is_empty() {
        return;
    }
    let title = slot
        .messages
        .last()
        .and_then(|m| m.operation.as_deref())
        .map_or_else(
            || UI_STRINGS.pad(UI_STRINGS.dialogs.notification),
            |s| UI_STRINGS.pad(s),
        );
    let lines: Vec<Line<'_>> = slot
        .messages
        .iter()
        .flat_map(|m| {
            let prefix = format!(" [{}] ", level_short(m.level));
            let indent = " ".repeat(prefix.len());
            let style = level_style(m.level).add_modifier(Modifier::BOLD);
            m.text
                .split('\n')
                .enumerate()
                .map(|(i, seg)| {
                    let p = if i == 0 { prefix.as_str() } else { &indent };
                    Line::from(Span::styled(format!("{p}{seg}"), style))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.focused_border).bg(t.notification_bg))
        .style(Style::default().bg(t.notification_bg))
        .title(title);
    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
