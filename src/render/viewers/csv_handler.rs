//! Render delimiter-separated file content in the viewer as a table with box-drawing borders
//! (comfy-table). **Delimiter:** extension on the viewed path first (`.csv` → comma, `.tsv`/`.tab` →
//! tab, `.psv` → pipe) via [`crate::integrations::delimiter_from_path_for_viewer`]; if the
//! extension doesn’t decide, fall back to zahirscan’s [`crate::integrations::detect_delimiter_byte`]
//! on the file contents.
//!
//! Layout matches markdown tables: word wrap, short columns without wrap (still row-padded), and
//! [`crate::render::viewers::pretty_tables::VIEWER_TABLE_ELLIPSIS_CELL_CHARS`] truncation with `"..."`.

use ratatui::text::Text;
use std::io::Cursor;

use crate::integrations::{delimiter_from_path_for_viewer, detect_delimiter_byte};
use crate::render::viewers::pretty_tables;

/// Parse with a single-byte delimiter. Uses the **`csv`** crate via `::csv::` (avoids confusion with
/// this module’s name, `csv_handler`).
fn parse_with_delimiter(raw: &str, delim: u8) -> Result<Vec<Vec<String>>, ::csv::Error> {
    let mut rows = Vec::new();
    let mut rdr = ::csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(delim)
        .from_reader(Cursor::new(raw));
    for result in rdr.records() {
        let record = result?;
        rows.push(record.iter().map(String::from).collect());
    }
    Ok(rows)
}

/// Parse raw delimiter-separated text into a grid (first row = header for [`table_string`]).
///
/// `path_hint` should be the viewed file path when known so extensions select the delimiter; if
/// [`None`] or the extension is unknown, zahirscan’s content sniffing is used.
///
/// # Errors
///
/// Returns [`csv::Error`] when a row cannot be read or parsed.
pub fn parse_csv(raw: &str, path_hint: Option<&str>) -> Result<Vec<Vec<String>>, ::csv::Error> {
    let hint = path_hint.unwrap_or("");
    let delim = delimiter_from_path_for_viewer(hint).unwrap_or_else(|| detect_delimiter_byte(raw));
    parse_with_delimiter(raw, delim)
}

/// Build a comfy-table string from parsed rows: UTF8 box-drawing style, cells truncated.
/// First row is the header; `content_width` constrains table width; cells capped at `max_cell_chars`.
#[must_use]
pub fn table_string_with_max_cell(
    rows: &[Vec<String>],
    content_width: u16,
    max_cell_chars: usize,
) -> String {
    pretty_tables::table_string_with_max_cell(rows, content_width, max_cell_chars)
}

/// Like [`table_string_with_max_cell`] but treats every row as a body row (no `set_header`).
/// Use for markdown tables so header and data each get their own line.
#[must_use]
pub fn table_string_rows_only(
    rows: &[Vec<String>],
    content_width: u16,
    max_cell_chars: usize,
) -> String {
    pretty_tables::table_string_rows_only(rows, content_width, max_cell_chars)
}

/// Build a comfy-table string from parsed rows (first row = header): smart wrap / ellipsis.
#[must_use]
pub fn table_string(rows: &[Vec<String>], content_width: u16) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut rows = rows.to_vec();
    let header = rows.remove(0);
    pretty_tables::table_string_header_body_smart_wrap(&header, &rows, content_width)
}

/// Table as styled [`Text`] for the viewer, using [`crate::themes::current`] text color for the whole table.
#[must_use]
pub fn table_to_text(rows: &[Vec<String>], content_width: u16) -> Text<'static> {
    table_string_to_text(&table_string(rows, content_width))
}

/// Turn a pre-rendered table string into styled [Text] (for cache path).
#[must_use]
pub fn table_string_to_text(table_str: &str) -> Text<'static> {
    pretty_tables::table_string_to_text(table_str)
}

/// Number of lines the table string will occupy (for scroll height).
#[must_use]
pub fn table_line_count(rows: &[Vec<String>], content_width: u16) -> usize {
    table_string(rows, content_width).lines().count()
}

/// Build table string and line count in one pass (for caching).
#[must_use]
pub fn table_string_and_line_count(rows: &[Vec<String>], content_width: u16) -> (String, usize) {
    let s = table_string(rows, content_width);
    let count = s.lines().count();
    (s, count)
}
