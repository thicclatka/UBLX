//! Status line: powerline node (Latest Snapshot) + Search: + Esc to clear, all on one line.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::layout::style;
use crate::ui::UI_STRINGS;
use crate::utils::format_timestamp_ns;

/// One line: powerline node "Latest Snapshot: <time>" (when Some) + " Search: <query>  Esc to clear " when search active or query non-empty.
pub(super) fn draw_status_line(
    f: &mut Frame,
    area: Rect,
    latest_snapshot_ns: Option<i64>,
    search_active: bool,
    search_query: &str,
) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if let Some(ns) = latest_snapshot_ns {
        let node_content = format!("{}: {}", UI_STRINGS.latest_snapshot_label, format_timestamp_ns(ns));
        spans.extend(style::status_node_spans(&node_content));
    }
    if search_active || !search_query.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{}{}", UI_STRINGS.status_search_label, search_query),
            style::search_text(),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            UI_STRINGS.status_esc_to_clear,
            style::hint_text(),
        ));
    }
    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}
