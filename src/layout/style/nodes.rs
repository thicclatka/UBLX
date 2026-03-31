//! Powerline-style nodes: tabs, footer lines, status spans.

use ratatui::layout::HorizontalAlignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::themes;
use crate::ui::{UI_GLYPHS, UI_STRINGS};

use super::{CurrentTheme, ThemeStyles, tab_active, tab_inactive};

/// One tab as a powerline-style node: round + " label " + round. No separator.
/// Used for right-pane tabs and main (Snapshot/Delta) tabs.
/// When `chord_mode` is true and `active`, uses hint-colored chrome (Ctrl chord leader).
#[must_use]
pub fn tab_node_segment(label: &str, active: bool, chord_mode: bool) -> Vec<Span<'static>> {
    let t = CurrentTheme::palette();
    let (circle_style, node_style) = if active {
        if chord_mode {
            (
                Style::default().fg(t.hint),
                Style::default().fg(t.background).bg(t.hint),
            )
        } else {
            (Style::default().fg(t.tab_active_bg), tab_active())
        }
    } else {
        (Style::default().fg(t.tab_inactive_bg), tab_inactive())
    };
    vec![
        Span::styled(UI_GLYPHS.round_left.to_string(), circle_style),
        Span::styled(format!(" {label} "), node_style),
        Span::styled(UI_GLYPHS.round_right.to_string(), circle_style),
    ]
}

fn node_color() -> (ratatui::style::Color, Style, Style) {
    let t = CurrentTheme::palette();
    let pill_bg = themes::node_pill_background(t);
    let circle_style = Style::default().fg(pill_bg).bg(t.background);
    let node_style = Style::default().fg(t.text).bg(pill_bg);
    (pill_bg, circle_style, node_style)
}

fn chord_node_color() -> (Style, Style) {
    let t = CurrentTheme::palette();
    let pill_bg = t.hint;
    let circle_style = Style::default().fg(pill_bg).bg(t.background);
    let node_style = Style::default().fg(t.background).bg(pill_bg);
    (circle_style, node_style)
}

fn node_spans(content: &str, circle_style: Style, node_style: Style) -> Vec<Span<'static>> {
    vec![
        Span::styled(UI_GLYPHS.round_left.to_string(), circle_style),
        Span::styled(format!(" {content} "), node_style),
        Span::styled(UI_GLYPHS.round_right.to_string(), circle_style),
    ]
}

/// Single-node footer line with the given alignment (e.g. viewer size = Right, categories "Latest Snapshot" = Left).
#[must_use]
pub fn node_line(text: &str, alignment: HorizontalAlignment, chord_mode: bool) -> Line<'static> {
    let (circle_style, node_style) = if chord_mode {
        chord_node_color()
    } else {
        let (_, c, n) = node_color();
        (c, n)
    };
    Line::from(node_spans(text, circle_style, node_style)).alignment(alignment)
}

/// Powerline-style node spans for use in a combined status line (e.g. Latest Snapshot, not in a border).
#[must_use]
pub fn status_node_spans(content: &str, chord_mode: bool) -> Vec<Span<'static>> {
    let (circle_style, node_style) = if chord_mode {
        chord_node_color()
    } else {
        let (_, c, n) = node_color();
        (c, n)
    };
    node_spans(content, circle_style, node_style)
}

/// Accent for [`popup_input_line_spans`] (label fg, solid bar): editing vs after Enter.
#[must_use]
pub fn popup_input_accent_color(submitted: bool) -> Color {
    let t = themes::current();
    if submitted { t.hint } else { t.focused_border }
}

/// One-line popup input strip: accent bar, `popup_bg` buffer space, hint-styled label, query text.
/// Shared by catalog search (status line) and viewer find (right pane `title_bottom`); placement is caller-specific.
///
/// Accent colors follow [`popup_input_accent_color`].
#[must_use]
pub fn popup_input_line_spans(
    label: impl Into<String>,
    query: impl Into<String>,
    submitted: bool,
) -> Vec<Span<'static>> {
    let t = themes::current();
    let bg = t.popup_bg;
    let accent = popup_input_accent_color(submitted);
    vec![
        Span::styled(" ", Style::default().bg(accent).fg(accent)),
        Span::styled(" ", Style::default().bg(bg)),
        Span::styled(label.into(), Style::default().fg(accent).bg(bg)),
        Span::styled(query.into(), Style::default().fg(t.search_text).bg(bg)),
    ]
}

/// Catalog `/` search on the status line. Callers hide the Latest Snapshot node while this is visible.
#[must_use]
pub fn search_catalog_popup_spans(
    search_active: bool,
    search_query: &str,
) -> Option<Vec<Span<'static>>> {
    let show = search_active || !search_query.trim().is_empty();
    if !show {
        return None;
    }
    Some(popup_input_line_spans(
        UI_STRINGS.search.search_label.to_string(),
        search_query.to_string(),
        !search_active,
    ))
}

/// Footer line: optional PDF page, optional size, and optional mtime — **right-aligned** together.
#[must_use]
pub fn viewer_footer_line(
    size_str: Option<&str>,
    mtime_ns: Option<i64>,
    pdf_page_line: Option<&str>,
    chord_mode: bool,
) -> Option<Line<'static>> {
    use crate::utils::format_timestamp_ns;
    let (circle_style, node_style) = if chord_mode {
        chord_node_color()
    } else {
        let (_, c, n) = node_color();
        (c, n)
    };
    let mut spans: Vec<Span<'static>> = Vec::new();
    if let Some(pdf) = pdf_page_line {
        spans.extend(node_spans(pdf, circle_style, node_style));
    }
    match (size_str, mtime_ns) {
        (Some(s), None) => {
            spans.extend(node_spans(s, circle_style, node_style));
        }
        (None, Some(ns)) => {
            spans.extend(node_spans(
                &format_timestamp_ns(ns),
                circle_style,
                node_style,
            ));
        }
        (Some(s), Some(ns)) => {
            spans.extend(node_spans(s, circle_style, node_style));
            spans.extend(node_spans(
                &format_timestamp_ns(ns),
                circle_style,
                node_style,
            ));
        }
        (None, None) if pdf_page_line.is_none() => return None,
        (None, None) => {}
    }
    Some(Line::from(spans).right_aligned())
}
