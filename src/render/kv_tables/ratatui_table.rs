//! Ratatui [`Table`] widgets for [`super::sections`] (metadata, writing, sheet-style JSON).
//!
//! Windowed row slicing and column width balancing live here; see [`super::draw`] for painting.
//! File-viewer grids (CSV / Markdown) use **comfy-table** in [`crate::render::viewers::pretty_tables`].

use ratatui::layout::Constraint;
use ratatui::widgets::{Cell, Row, Table};
use rayon::prelude::*;

use super::{
    format,
    sections::{ContentsSection, KvSection, SingleColumnListSection},
};
use crate::config::PARALLEL;
use crate::layout::style;
use crate::ui::UI_STRINGS;
use crate::utils::truncate_middle;

const COLUMN_SPACING: usize = 1;
const KEY_WIDTH_FALLBACK: usize = 4;
const KEY_WIDTH_MIN: usize = 35;
const VALUE_WIDTH_MIN: usize = 10;

/// When a table has more than this many columns, we balance widths to fill the pane; otherwise we use natural (compact) widths so few-column tables (e.g. sheet stats) don’t look over-spaced.
const SIZE_OPTIMIZATION_COLUMN_THRESHOLD: usize = 3;

/// Compute column widths (in characters) from natural widths and available width.
/// Natural width per column is typically max(header len, max cell len in column).
/// If total natural fits, use natural (capped by available); otherwise scale down
/// proportionally. Distribute any remainder so sum equals available. Each column gets at least 1.
pub fn balanced_column_widths(
    natural: &[usize],
    available_width: usize,
    spacing: usize,
) -> Vec<u16> {
    let n = natural.len().max(1);
    let gaps = (n - 1) * spacing;
    let available = available_width.saturating_sub(gaps);
    if available == 0 {
        return natural.iter().map(|_| 1u16).collect();
    }
    let total: usize = natural.iter().sum();
    if total == 0 {
        let w = (available / n).min(u16::MAX as usize) as u16;
        return (0..natural.len()).map(|_| w.max(1)).collect();
    }
    let mut widths: Vec<u16> = natural
        .iter()
        .map(|&nat| {
            let w = (nat * available) / total;
            (w.min(u16::MAX as usize).max(1)) as u16
        })
        .collect();
    let mut remainder = available.saturating_sub(widths.iter().map(|&w| w as usize).sum::<usize>());
    for w in widths.iter_mut() {
        if remainder == 0 {
            break;
        }
        *w = (*w as usize + 1).min(u16::MAX as usize) as u16;
        remainder -= 1;
    }
    widths
}

pub fn entry_cell(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key)
        .map(|v| format::format_value(v, key))
        .unwrap_or_else(|| "—".to_string())
}

/// Build key/value table for one section.
pub fn section_to_table(section: &KvSection, row_offset: usize) -> Table<'_> {
    let header = Row::new(vec![
        UI_STRINGS.table_header_key,
        UI_STRINGS.table_header_value,
    ])
    .style(style::table_header_style())
    .bottom_margin(0);
    let data_rows: Vec<Row> = section
        .rows
        .iter()
        .enumerate()
        .map(|(i, (k, v))| {
            let value_cell = match format::value_cell_style(v.as_str()) {
                Some(s) => Cell::from(ratatui::text::Line::from(v.as_str()).style(s)),
                None => Cell::from(v.as_str()),
            };
            Row::new(vec![Cell::from(k.as_str()), value_cell])
                .style(style::table_row_style(row_offset + i))
        })
        .collect();
    let key_w = section
        .rows
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(KEY_WIDTH_FALLBACK)
        .min(KEY_WIDTH_MIN) as u16;
    Table::new(
        data_rows,
        [
            Constraint::Length(key_w),
            Constraint::Min(VALUE_WIDTH_MIN as u16),
        ],
    )
    .header(header)
    .column_spacing(1)
    .style(style::text_style())
}

/// Build one display row; string values are truncated to fit column width (chars).
fn contents_row(
    obj: &serde_json::Map<String, serde_json::Value>,
    column_keys: &[String],
    column_widths: &[u16],
) -> Vec<String> {
    column_keys
        .iter()
        .enumerate()
        .map(|(j, k)| {
            let cell = entry_cell(obj, k);
            let max_chars = column_widths.get(j).copied().unwrap_or(0) as usize;
            let len = cell.chars().count();
            if max_chars > 0 && len > max_chars {
                truncate_middle(&cell, max_chars)
            } else {
                cell
            }
        })
        .collect()
}

/// Natural width (chars) per column: max of header length and max cell length in visible window.
/// Column names (headers) are always included so they are never squeezed.
/// Uses parallel iteration when visible row count exceeds [PARALLEL.contents_natural_widths].
fn contents_natural_widths(section: &ContentsSection, start: usize, end: usize) -> Vec<usize> {
    let keys = &section.column_keys;
    let cols = &section.columns;
    if keys.is_empty() {
        return vec![];
    }
    let header_natural: Vec<usize> = cols.iter().map(|s| s.chars().count()).collect();
    let entries_window = end.saturating_sub(start);
    if entries_window < PARALLEL.contents_natural_widths {
        let mut natural = header_natural;
        for v in section.entries.iter().skip(start).take(entries_window) {
            let Some(obj) = v.as_object() else { continue };
            for (j, k) in keys.iter().enumerate() {
                let len = entry_cell(obj, k).chars().count();
                if let Some(nat) = natural.get_mut(j) {
                    *nat = (*nat).max(len);
                }
            }
        }
        natural
    } else {
        let slice = &section.entries[start..end];
        let chunk_size = (entries_window / 4).max(1);
        let chunk_naturals: Vec<Vec<usize>> = slice
            .par_chunks(chunk_size)
            .map(|chunk| {
                let mut nat = header_natural.clone();
                for v in chunk {
                    let Some(obj) = v.as_object() else { continue };
                    for (j, k) in keys.iter().enumerate() {
                        let len = entry_cell(obj, k).chars().count();
                        if let Some(nat_j) = nat.get_mut(j) {
                            *nat_j = (*nat_j).max(len);
                        }
                    }
                }
                nat
            })
            .collect();
        let mut natural = header_natural;
        for chunk_nat in chunk_naturals {
            for (j, &cn) in chunk_nat.iter().enumerate() {
                if let Some(nat_j) = natural.get_mut(j) {
                    *nat_j = (*nat_j).max(cn);
                }
            }
        }
        natural
    }
}

/// Minimum width per column (header length) so column names are never truncated.
fn contents_header_widths(section: &ContentsSection) -> Vec<u16> {
    section
        .columns
        .iter()
        .map(|s| s.chars().count().min(u16::MAX as usize) as u16)
        .collect()
}

/// Build multi-column table for a Contents section, only for entry indices [start, end) (for virtualization).
/// Column widths are derived from content (header + visible rows), balanced against `table_width`.
pub fn contents_to_table_window(
    section: &ContentsSection,
    row_offset: usize,
    start: usize,
    end: usize,
    table_width: u16,
) -> Table<'_> {
    let natural = contents_natural_widths(section, start, end);
    let header_widths = contents_header_widths(section);
    let ncols = section.column_keys.len();
    let use_size_optimization = ncols > SIZE_OPTIMIZATION_COLUMN_THRESHOLD;

    let mut column_widths = if natural.is_empty() {
        let available =
            (table_width as usize).saturating_sub((ncols.saturating_sub(1)) * COLUMN_SPACING);
        let w = (available / ncols.max(1)).min(u16::MAX as usize) as u16;
        (0..ncols).map(|_| w.max(1)).collect::<Vec<u16>>()
    } else if use_size_optimization {
        balanced_column_widths(&natural, table_width as usize, COLUMN_SPACING)
    } else {
        let gaps = (ncols.saturating_sub(1)) * COLUMN_SPACING;
        let natural_with_header: Vec<usize> = natural
            .iter()
            .zip(header_widths.iter())
            .map(|(n, &hw)| (*n).max(hw as usize))
            .collect();
        let total_compact = natural_with_header.iter().sum::<usize>() + gaps;
        if total_compact <= table_width as usize {
            natural_with_header
                .into_iter()
                .map(|w| w.min(u16::MAX as usize) as u16)
                .collect()
        } else {
            balanced_column_widths(&natural_with_header, table_width as usize, COLUMN_SPACING)
        }
    };
    for (j, &min_w) in header_widths.iter().enumerate() {
        if let Some(w) = column_widths.get_mut(j) {
            *w = (*w).max(min_w);
        }
    }
    let constraints: Vec<Constraint> = column_widths
        .iter()
        .map(|&w| Constraint::Length(w))
        .collect();

    let header = Row::new(
        section
            .columns
            .iter()
            .map(|s| Cell::from(s.as_str()))
            .collect::<Vec<_>>(),
    )
    .style(style::table_header_style())
    .bottom_margin(0);
    let data_rows: Vec<Row> = section
        .entries
        .iter()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
        .filter_map(|(_i, v)| v.as_object())
        .enumerate()
        .map(|(idx, obj)| {
            let row_strs = contents_row(obj, &section.column_keys, &column_widths);
            Row::new(row_strs.into_iter().map(Cell::from).collect::<Vec<_>>())
                .style(style::table_row_style(row_offset + start + idx))
        })
        .collect();
    Table::new(data_rows, constraints)
        .header(header)
        .column_spacing(1)
        .style(style::text_style())
}

/// Build a single-column table with no header (e.g. common_pivots list). Only rows [start, end) are included.
pub fn single_column_list_to_table(
    section: &SingleColumnListSection,
    row_offset: usize,
    start: usize,
    end: usize,
) -> Table<'_> {
    let data_rows: Vec<Row> = section
        .values
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .enumerate()
        .map(|(idx, s)| {
            Row::new(vec![Cell::from(s.as_str())])
                .style(style::table_row_style(row_offset + start + idx))
        })
        .collect();
    Table::new(data_rows, [Constraint::Min(0)])
        .column_spacing(1)
        .style(style::text_style())
}
