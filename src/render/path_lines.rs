//! Wrap long path strings for narrow panes: line breaks prefer the last `/` or `\` in each segment.

use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Wraps `s` to fit `width` terminal columns. When a segment must split, the break is placed after the
/// last `/` or `\` in the visible window when possible; continuation lines are prefixed with
/// `continuation_indent`, then styled with `line_style`.
#[must_use]
pub fn wrap_lines_at_path_separators(
    s: &str,
    width: usize,
    continuation_indent: &str,
    line_style: Style,
) -> Vec<Line<'static>> {
    wrap_path_string_segments(s, width, continuation_indent)
        .into_iter()
        .map(|text| Line::from(Span::styled(text, line_style)))
        .collect()
}

/// Same layout algorithm as [`wrap_lines_at_path_separators`], without styling — one string per visual line.
#[must_use]
pub fn wrap_path_string_segments(s: &str, width: usize, continuation_indent: &str) -> Vec<String> {
    let width = width.max(1);
    if s.is_empty() {
        return vec![String::new()];
    }

    let indent_w = continuation_indent.chars().count();
    let mut out: Vec<String> = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut start = 0usize;

    while start < chars.len() {
        let is_first = out.is_empty();
        let max_content = if is_first {
            width
        } else {
            width.saturating_sub(indent_w).max(1)
        };

        let remaining = chars.len() - start;
        if remaining <= max_content {
            let piece: String = chars[start..].iter().collect();
            out.push(if is_first {
                piece
            } else {
                format!("{continuation_indent}{piece}")
            });
            break;
        }

        let end = (start + max_content).min(chars.len());
        let window = &chars[start..end];
        let rel_break = window.iter().rposition(|c| *c == '/' || *c == '\\');

        let break_at = if let Some(rel) = rel_break {
            start + rel + 1
        } else {
            start + max_content
        };

        let piece: String = chars[start..break_at].iter().collect();
        out.push(if is_first {
            piece
        } else {
            format!("{continuation_indent}{piece}")
        });
        start = break_at;
    }

    out
}
