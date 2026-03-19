use ratatui::text::Line;

use crate::render::viewers::pretty_tables::{
    self, collapse_viewer_cell_whitespace, pad_row_to_cols,
};

fn is_separator_row(row: &[String]) -> bool {
    let is_separator_cell = |s: &str| {
        let t = s.trim().trim_matches('|').trim();
        !t.is_empty() && t.chars().all(|c| c == '-' || c == ':' || c.is_whitespace())
    };
    !row.is_empty() && row.iter().all(|c| is_separator_cell(c))
}

/// Strip GFM separator rows; collapse `\n` inside cells into spaces.
fn markdown_table_body_rows(rows: &[Vec<String>]) -> Vec<Vec<String>> {
    rows.iter()
        .filter(|row| !row.is_empty())
        .map(|row| {
            row.iter()
                .map(|c| collapse_viewer_cell_whitespace(c))
                .collect()
        })
        .filter(|row: &Vec<String>| !is_separator_row(row))
        .collect()
}

#[must_use]
pub fn render_markdown_table_lines(
    header: &[String],
    rows: &[Vec<String>],
    width: u16,
) -> Vec<Line<'static>> {
    let data_rows = markdown_table_body_rows(rows);
    let col_count = header
        .len()
        .max(data_rows.iter().map(|r| r.len()).max().unwrap_or(0))
        .max(1);

    let header_cells = pad_row_to_cols(header, col_count);
    let body: Vec<Vec<String>> = data_rows
        .iter()
        .map(|r| pad_row_to_cols(r, col_count))
        .collect();

    let table_str = pretty_tables::table_string_header_body_smart_wrap(&header_cells, &body, width);
    pretty_tables::table_string_to_lines(&table_str)
}
