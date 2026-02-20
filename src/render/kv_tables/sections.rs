//! Section types and JSON parsing into Key/Value and Contents sections.

use serde_json::Value;

use crate::ui::UI_STRINGS;

use super::consts::TABLE_GAP;
use super::csv;
use super::walk;

/// One key/value section: optional title and rows (key, value).
pub struct KvSection {
    pub title: Option<String>,
    pub rows: Vec<(String, String)>,
}

/// Multi-column table section (e.g. zip entries "Contents"). Stores raw entries for virtualization; only visible rows are built when drawing.
pub struct ContentsSection {
    pub title: String,
    pub columns: Vec<String>,
    pub column_keys: Vec<String>,
    pub entries: Vec<Value>,
}

/// Single-column list with no header (e.g. common_pivots, schema tree).
pub struct SingleColumnListSection {
    pub title: String,
    pub values: Vec<String>,
}

/// Either a key/value table, a multi-column contents table, or a single-column list.
pub enum Section {
    KeyValue(KvSection),
    Contents(ContentsSection),
    SingleColumnList(SingleColumnListSection),
}

/// Parse JSON string (one or more objects joined by "\n\n") into sections. First section is titled "General"; nested objects become separate sections; objects with "entries" get an extra "Contents" table. Special keys: schema (tree), sheet_stats, common_pivots, csv_metadata.
pub fn parse_json_sections(json: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let blobs: Vec<&str> = json
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .collect();

    for blob in blobs {
        let value: Value = match serde_json::from_str(blob.trim()) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let map = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        if csv::is_csv_metadata(map) {
            sections.extend(csv::sections_from_csv_root(map));
            continue;
        }

        walk::push_root_parts(&mut sections, map);
    }

    if let Some(Section::KeyValue(kv)) = sections.first_mut() {
        kv.title = Some(UI_STRINGS.first_table_title.to_string());
    }
    sections
}

/// Total line count for the parsed sections (title + header + data rows + gaps). Used for scrollbar and clamping.
pub fn content_height(json: &str) -> u16 {
    let sections = parse_json_sections(json);
    if sections.is_empty() {
        return 0;
    }
    let mut lines: u16 = 0;
    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            lines += TABLE_GAP;
        }
        let (has_title, header_lines, num_rows) = match section {
            Section::KeyValue(kv) => (kv.title.is_some(), 1, kv.rows.len()),
            Section::Contents(c) => (true, 1, c.entries.len()),
            Section::SingleColumnList(list) => (true, 0, list.values.len()),
        };
        lines += if has_title { 1 } else { 0 };
        lines += header_lines as u16;
        lines += num_rows as u16;
    }
    lines
}
