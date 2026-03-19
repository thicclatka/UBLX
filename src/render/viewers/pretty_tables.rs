//! Comfy-table (Unicode box drawing) for the file viewer: CSV, Markdown, and shared layout helpers.

use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use rayon::prelude::*;

use crate::layout::themes;

// -----------------------------------------------------------------------------
// Truncation helpers (legacy explicit max, parallel path)
// -----------------------------------------------------------------------------

/// Minimum row count to use parallel truncation (avoids rayon overhead on small tables).
const PARALLEL_TRUNCATE_THRESHOLD: usize = 100;

fn truncate_cell(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

/// Truncate all cells in parallel (when many rows) or sequentially.
fn truncate_all_cells(rows: &[Vec<String>], max_chars: usize) -> Vec<Vec<String>> {
    let truncate_row = |row: &Vec<String>| {
        row.iter()
            .map(|c| truncate_cell(c, max_chars))
            .collect::<Vec<_>>()
    };
    if rows.len() >= PARALLEL_TRUNCATE_THRESHOLD {
        rows.par_iter().map(truncate_row).collect()
    } else {
        rows.iter().map(truncate_row).collect()
    }
}

/// Per-cell truncation cap derived from the viewer width and number of columns.
///
/// Comfy-table still receives `set_width(content_width)`; this limits how much text we keep per
/// cell before `"..."` so wide many-column tables don’t over‑truncate when the pane is narrow,
/// and single-column views can use more of a wide pane (up to `ceiling`).
pub fn max_cell_chars_for_viewport(
    content_width: u16,
    col_count: usize,
    floor: usize,
    ceiling: usize,
) -> usize {
    let cols = col_count.max(1);
    let w = content_width as usize;
    if w == 0 {
        return ceiling;
    }
    let per_col = w / cols;
    // Borders / padding eat a few terminal columns per cell in UTF-8 box-draw mode.
    let fudge = 4usize;
    per_col.saturating_sub(fudge).clamp(floor, ceiling)
}

fn base_table(content_width: u16) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(content_width);
    table
}

// -----------------------------------------------------------------------------
// Smart wrap + ellipsis (CSV + markdown): proportional widths, short columns, padding
// -----------------------------------------------------------------------------

/// Columns whose **maximum** cell length (characters) is ≤ this use one line per cell (no word wrap).
/// Shorter cells in other columns still get blank lines so row heights align.
pub const VIEWER_TABLE_NO_WRAP_COL_MAX_CHARS: usize = 14;

/// Cells longer than this are shown as a single truncated line with [`truncate_cell`] / `"..."`.
pub const VIEWER_TABLE_ELLIPSIS_CELL_CHARS: usize = 512;

/// Collapse embedded newlines / whitespace (CSV cells with `\n`, markdown normalization).
#[must_use]
pub fn collapse_viewer_cell_whitespace(cell: &str) -> String {
    cell.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[must_use]
pub fn pad_row_to_cols(row: &[String], col_count: usize) -> Vec<String> {
    let mut r: Vec<String> = row.to_vec();
    if r.len() > col_count {
        r.truncate(col_count);
    }
    r.resize_with(col_count, String::new);
    r
}

/// Allocate per-column wrap widths: proportional to (capped) longest cell per column.
#[must_use]
pub fn column_wrap_widths(viewport: u16, col_count: usize, max_lens: &[usize]) -> Vec<usize> {
    const MIN_WRAP: usize = 4;
    let cols = col_count.max(1);
    let w = viewport as usize;
    if w <= 2 {
        return vec![MIN_WRAP; cols];
    }
    let border_overhead = 1usize.saturating_add(cols.saturating_mul(3));
    let inner = w.saturating_sub(border_overhead).max(cols * MIN_WRAP);
    let weights: Vec<usize> = max_lens.iter().copied().map(|m| m.max(1)).collect();
    let sum_w: usize = weights.iter().sum();
    let mut widths: Vec<usize> = weights
        .iter()
        .map(|&wt| (inner * wt / sum_w).max(MIN_WRAP))
        .collect();
    let total: usize = widths.iter().sum();
    if total > inner {
        let mut excess = total - inner;
        while excess > 0 {
            let Some((i, _)) = widths.iter().enumerate().max_by_key(|(_, x)| *x) else {
                break;
            };
            if widths[i] > MIN_WRAP {
                widths[i] -= 1;
                excess -= 1;
            } else {
                break;
            }
        }
    } else if total < inner
        && let Some((i, _)) = widths.iter().enumerate().max_by_key(|(_, x)| *x)
    {
        widths[i] += inner - total;
    }
    widths
}

/// Word-wrap at whitespace; lines are at most `max_chars` Unicode scalar characters.
#[must_use]
pub fn word_wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![String::new()];
    }
    if max_chars == 0 {
        return vec![text.to_string()];
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    let flush = |current: &mut String, lines: &mut Vec<String>| {
        if !current.is_empty() {
            lines.push(std::mem::take(current));
        }
    };

    for word in text.split_whitespace() {
        let wl = word.chars().count();
        let cur_len = current.chars().count();
        let with_space = if cur_len > 0 { 1 } else { 0 };
        let needed = with_space + wl;

        if cur_len + needed <= max_chars {
            if with_space == 1 {
                current.push(' ');
            }
            current.push_str(word);
        } else {
            flush(&mut current, &mut lines);
            if wl > max_chars {
                let mut remaining: String = word.to_string();
                while !remaining.is_empty() {
                    let chunk: String = remaining.chars().take(max_chars).collect();
                    let n = chunk.chars().count();
                    remaining = remaining.chars().skip(n).collect();
                    if remaining.is_empty() {
                        current = chunk;
                    } else {
                        lines.push(chunk);
                    }
                }
            } else {
                current.push_str(word);
            }
        }
    }
    flush(&mut current, &mut lines);
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn cell_display_lines(
    text: &str,
    wrap_width: usize,
    column_max_content_chars: usize,
) -> Vec<String> {
    let t = collapse_viewer_cell_whitespace(text);
    let len = t.chars().count();
    if len > VIEWER_TABLE_ELLIPSIS_CELL_CHARS {
        let cap = wrap_width.max(8);
        return vec![truncate_cell(&t, cap)];
    }
    if column_max_content_chars <= VIEWER_TABLE_NO_WRAP_COL_MAX_CHARS {
        return if t.is_empty() {
            vec![String::new()]
        } else {
            vec![t]
        };
    }
    word_wrap_text(&t, wrap_width)
}

fn pad_row_cells(wrapped: Vec<Vec<String>>, target_lines: usize) -> Vec<String> {
    wrapped
        .into_iter()
        .map(|mut lines| {
            while lines.len() < target_lines {
                lines.push(String::new());
            }
            lines.join("\n")
        })
        .collect()
}

/// Build header row and body rows as multiline cell strings for [`table_string_multiline`].
#[must_use]
pub fn prepare_multiline_grid(
    header: &[String],
    body: &[Vec<String>],
    content_width: u16,
) -> (Vec<String>, Vec<Vec<String>>) {
    let col_count = header
        .len()
        .max(body.iter().map(|r| r.len()).max().unwrap_or(0))
        .max(1);

    let header_cells = pad_row_to_cols(header, col_count);
    let body: Vec<Vec<String>> = body.iter().map(|r| pad_row_to_cols(r, col_count)).collect();

    let max_lens: Vec<usize> = (0..col_count)
        .map(|j| {
            let h_len = collapse_viewer_cell_whitespace(
                header_cells.get(j).map(String::as_str).unwrap_or(""),
            )
            .chars()
            .count();
            let b_len = body
                .iter()
                .map(|r| {
                    collapse_viewer_cell_whitespace(r[j].as_str())
                        .chars()
                        .count()
                })
                .max()
                .unwrap_or(0);
            h_len.max(b_len)
        })
        .collect();

    let lens_for_widths: Vec<usize> = max_lens
        .iter()
        .copied()
        .map(|m| m.min(VIEWER_TABLE_ELLIPSIS_CELL_CHARS))
        .collect();

    let wrap_widths = column_wrap_widths(content_width, col_count, &lens_for_widths);

    let header_wrapped: Vec<Vec<String>> = header_cells
        .iter()
        .enumerate()
        .map(|(j, s)| cell_display_lines(s, wrap_widths[j], max_lens[j]))
        .collect();
    let header_h = header_wrapped.iter().map(|v| v.len()).max().unwrap_or(1);
    let comfy_header = pad_row_cells(header_wrapped, header_h);

    let mut comfy_body: Vec<Vec<String>> = Vec::with_capacity(body.len());
    for row in &body {
        let wrapped: Vec<Vec<String>> = row
            .iter()
            .enumerate()
            .map(|(j, s)| cell_display_lines(s, wrap_widths[j], max_lens[j]))
            .collect();
        let row_h = wrapped.iter().map(|v| v.len()).max().unwrap_or(1);
        comfy_body.push(pad_row_cells(wrapped, row_h));
    }

    (comfy_header, comfy_body)
}

// -----------------------------------------------------------------------------
// Public comfy-table string builders
// -----------------------------------------------------------------------------

/// Build a comfy-table string from parsed rows (first row as header), truncating cells.
pub fn table_string_with_max_cell(
    rows: &[Vec<String>],
    content_width: u16,
    max_cell_chars: usize,
) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut truncated = truncate_all_cells(rows, max_cell_chars);
    let mut table = base_table(content_width);
    let header = truncated.remove(0);
    table.set_header(header);
    for row in truncated {
        table.add_row(row);
    }
    table.to_string()
}

/// CSV / markdown: proportional word wrap, short-column no-wrap, ellipsis beyond
/// [`VIEWER_TABLE_ELLIPSIS_CELL_CHARS`], then comfy-table.
#[must_use]
pub fn table_string_header_body_smart_wrap(
    header: &[String],
    body: &[Vec<String>],
    content_width: u16,
) -> String {
    let (h, b) = prepare_multiline_grid(header, body, content_width);
    table_string_multiline(h, &b, content_width)
}

/// Build a comfy-table from a header row and body rows without truncating. Cells may contain
/// `\n` for multi-line content; comfy-table expands row height and aligns columns.
pub fn table_string_multiline(
    header: Vec<String>,
    body: &[Vec<String>],
    content_width: u16,
) -> String {
    let mut table = base_table(content_width);
    if !header.is_empty() {
        table.set_header(header);
    }
    for row in body {
        table.add_row(row.clone());
    }
    table.to_string()
}

/// Build a comfy-table string from parsed rows, treating all rows as body rows (no header).
pub fn table_string_rows_only(
    rows: &[Vec<String>],
    content_width: u16,
    max_cell_chars: usize,
) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let truncated = truncate_all_cells(rows, max_cell_chars);
    let mut table = base_table(content_width);
    for row in truncated {
        table.add_row(row);
    }
    table.to_string()
}

/// Convert a pre-rendered table string into lines styled with theme text color.
pub fn table_string_to_lines(table_str: &str) -> Vec<Line<'static>> {
    let style = Style::default().fg(themes::current().text);
    table_str
        .lines()
        .map(|l| Line::from(Span::styled(l.to_string(), style)))
        .collect()
}

/// Convert a pre-rendered table string into styled text.
pub fn table_string_to_text(table_str: &str) -> Text<'static> {
    Text::from(table_string_to_lines(table_str))
}
