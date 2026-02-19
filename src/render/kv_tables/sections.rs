//! Section types and JSON parsing into Key/Value and Contents sections.

use super::format;

/// Title for the first table when rendering JSON sections (metadata, writing).
pub const FIRST_TABLE_TITLE: &str = "General";
/// Blank lines between stacked tables.
pub const TABLE_GAP: u16 = 1;

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
    pub entries: Vec<serde_json::Value>,
}

/// Either a key/value table or a multi-column contents table.
pub enum Section {
    KeyValue(KvSection),
    Contents(ContentsSection),
}

fn push_contents_from_entries(sections: &mut Vec<Section>, arr: Vec<serde_json::Value>) {
    let objs: Vec<&serde_json::Map<String, serde_json::Value>> =
        arr.iter().filter_map(|v| v.as_object()).collect();
    if let Some(first) = objs.first() {
        let column_keys: Vec<String> = first.keys().cloned().collect();
        let columns: Vec<String> = column_keys.iter().map(|k| format::format_key(k)).collect();
        let entries: Vec<serde_json::Value> =
            arr.iter().filter(|v| v.is_object()).cloned().collect();
        if !entries.is_empty() {
            sections.push(Section::Contents(ContentsSection {
                title: "Contents".to_string(),
                columns,
                column_keys,
                entries,
            }));
        }
    }
}

/// Parse JSON string (one or more objects joined by "\n\n") into sections. First section is titled "General"; nested objects become separate sections; objects with "entries" get an extra "Contents" table.
pub fn parse_json_sections(json: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let blobs: Vec<&str> = json
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .collect();
    for blob in blobs {
        let value: serde_json::Value = match serde_json::from_str(blob.trim()) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };
        let mut flat = Vec::new();
        let mut nested = Vec::<(String, serde_json::Map<String, serde_json::Value>)>::new();
        let mut root_entries: Option<Vec<serde_json::Value>> = None;
        for (k, v) in obj {
            if k == "entries" {
                if let serde_json::Value::Array(arr) = v {
                    root_entries = Some(arr.clone());
                }
                continue;
            }
            match v {
                serde_json::Value::Object(map) if !map.is_empty() => {
                    nested.push((k.clone(), map.clone()));
                }
                _ => {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
        }
        if !flat.is_empty() {
            sections.push(Section::KeyValue(KvSection {
                title: None,
                rows: flat,
            }));
        }
        for (section_key, map) in nested {
            let title = format::format_key(&section_key);
            let mut kv_rows: Vec<(String, String)> = Vec::new();
            let mut entries_for_contents: Option<Vec<serde_json::Value>> = None;
            for (k, v) in &map {
                if k == "entries" {
                    if let serde_json::Value::Array(arr) = v {
                        entries_for_contents = Some(arr.clone());
                    }
                    continue;
                }
                kv_rows.push((format::format_key(k), format::format_value(v, k)));
            }
            if !kv_rows.is_empty() {
                sections.push(Section::KeyValue(KvSection {
                    title: Some(title),
                    rows: kv_rows,
                }));
            }
            if let Some(arr) = entries_for_contents {
                push_contents_from_entries(&mut sections, arr);
            }
        }
        if let Some(arr) = root_entries {
            push_contents_from_entries(&mut sections, arr);
        }
    }
    if let Some(Section::KeyValue(kv)) = sections.first_mut() {
        kv.title = Some(FIRST_TABLE_TITLE.to_string());
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
        let (has_title, num_rows) = match section {
            Section::KeyValue(kv) => (kv.title.is_some(), kv.rows.len()),
            Section::Contents(c) => (true, c.entries.len()),
        };
        lines += if has_title { 1 } else { 0 };
        lines += 1; // header
        lines += num_rows as u16;
    }
    lines
}
