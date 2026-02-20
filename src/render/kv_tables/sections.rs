//! Section types and JSON parsing into Key/Value and Contents sections.

use serde_json::{Map, Value};

use crate::ui::{TREE_CHARS, UI_STRINGS};

use super::csv::{self, MetadataArrayKeySliceExt};
use super::format;
use super::xlsx;

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

/// JSON keys that trigger special section handling (not treated as plain key/value or nested subsection).
pub struct SectionKeys;
impl SectionKeys {
    pub const SCHEMA: &'static str = "schema";
    pub const ENTRIES: &'static str = "entries";
    pub const SHEET_STATS: &'static str = "sheet_stats";
    pub const COMMON_PIVOTS: &'static str = "common_pivots";
    pub const CSV_METADATA: &'static str = "csv_metadata";
}

pub struct SchemaKeys;
impl SchemaKeys {
    pub const ATTRIBUTES: &'static str = "attributes";
    pub const CHILDREN: &'static str = "children";

    pub fn has_attributes(map: &Map<String, Value>) -> bool {
        map.contains_key(SchemaKeys::ATTRIBUTES)
    }

    pub fn has_children(map: &Map<String, Value>) -> bool {
        map.contains_key(SchemaKeys::CHILDREN)
    }

    #[inline]
    pub fn has_children_or_attributes(map: &Map<String, Value>) -> bool {
        SchemaKeys::has_attributes(map) || SchemaKeys::has_children(map)
    }
}

/// Returns (branch_line_prefix, continuation_prefix) for the next level. Use branch for the current line, continuation for recursing.
fn tree_prefixes(continuation: &str, is_last: bool) -> (String, String) {
    let branch = if is_last {
        TREE_CHARS.last_branch
    } else {
        TREE_CHARS.branch
    };
    let cont = if is_last {
        TREE_CHARS.space
    } else {
        TREE_CHARS.vertical
    };
    (
        format!("{continuation}{branch}"),
        format!("{continuation}{cont}"),
    )
}

fn push_contents_from_entries(sections: &mut Vec<Section>, arr: Vec<Value>) {
    let objs: Vec<&Map<String, Value>> = arr.iter().filter_map(|v| v.as_object()).collect();
    if let Some(first) = objs.first() {
        let column_keys: Vec<String> = first.keys().cloned().collect();
        let columns: Vec<String> = column_keys.iter().map(|k| format::format_key(k)).collect();
        let entries: Vec<Value> = arr.iter().filter(|v| v.is_object()).cloned().collect();
        if !entries.is_empty() {
            sections.push(Section::Contents(ContentsSection {
                title: UI_STRINGS.contents_table_title.to_string(),
                columns,
                column_keys,
                entries,
            }));
        }
    }
}

/// Walk one schema node; label is the node name (from parent key or root key). Supports XML-style (attributes/children objects) and TOML-style (nested key-value). Skips empty attributes and empty children.
fn schema_node_lines(
    value: &Value,
    line_prefix: &str,
    continuation: &str,
    label: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    if let Value::Object(map) = value {
        if map.is_empty() {
            out.push(format!("{line_prefix}{label}"));
            return out;
        }
        if SchemaKeys::has_children_or_attributes(map) {
            out.push(format!("{line_prefix}{label}"));
            let children = map.get(SchemaKeys::CHILDREN).and_then(Value::as_object);
            let has_children = children.is_some_and(|c| !c.is_empty());
            if let Some(attrs) = map.get(SchemaKeys::ATTRIBUTES).and_then(Value::as_object)
                && !attrs.is_empty()
            {
                let n_attrs = attrs.len();
                for (i, (k, v)) in attrs.iter().enumerate() {
                    let is_last = i == n_attrs.saturating_sub(1) && !has_children;
                    let (branch_prefix, _) = tree_prefixes(continuation, is_last);
                    out.push(format!(
                        "{} {}: {}",
                        branch_prefix,
                        format::format_key(k),
                        format::value_to_string(v)
                    ));
                }
            }
            if let Some(children_map) = children
                && !children_map.is_empty()
            {
                let n = children_map.len();
                for (i, (child_name, child_val)) in children_map.iter().enumerate() {
                    let (child_line, child_cont) =
                        tree_prefixes(continuation, i == n.saturating_sub(1));
                    out.extend(schema_node_lines(
                        child_val,
                        &child_line,
                        &child_cont,
                        child_name,
                    ));
                }
            }
            return out;
        }
        out.push(format!("{line_prefix}{label}"));
        let n = map.len();
        for (i, (k, v)) in map.iter().enumerate() {
            let (child_line, child_cont) = tree_prefixes(continuation, i == n.saturating_sub(1));
            match v {
                Value::Object(m) if !m.is_empty() => {
                    out.extend(schema_node_lines(v, &child_line, &child_cont, k));
                }
                _ => {
                    out.push(format!(
                        "{}{}: {}",
                        child_line,
                        format::format_key(k),
                        format::value_to_string(v)
                    ));
                }
            }
        }
    } else {
        out.push(format!(
            "{line_prefix}{label}: {}",
            format::value_to_string(value)
        ));
    }
    out
}

fn schema_value_to_list(value: &Value) -> Vec<String> {
    let lines = match value {
        Value::Object(map) if !map.is_empty() => {
            let mut lines = Vec::new();
            for (idx, (name, node_val)) in map.iter().enumerate() {
                if idx > 0 {
                    lines.push(String::new());
                }
                lines.extend(schema_node_lines(node_val, "", "", name));
            }
            lines
        }
        Value::Array(arr) if !arr.is_empty() => arr
            .iter()
            .flat_map(|v| {
                let label = v
                    .as_object()
                    .and_then(|o| {
                        o.get("name")
                            .or_else(|| o.get("type"))
                            .or_else(|| o.get("id"))
                            .and_then(Value::as_str)
                    })
                    .unwrap_or("…");
                schema_node_lines(v, "", "", label)
            })
            .collect::<Vec<_>>(),
        _ => schema_node_lines(value, "", "", "…"),
    };
    if lines.is_empty() && !value.is_null() {
        vec!["—".to_string()]
    } else {
        lines
    }
}

/// Parse JSON string (one or more objects joined by "\n\n") into sections. First section is titled "General"; nested objects become separate sections; objects with "entries" get an extra "Contents" table. Special keys: schema (tree), sheet_stats, common_pivots, csv_metadata.
pub fn parse_json_sections(json: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
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

        // When the blob is the csv_metadata object itself (no "csv_metadata" key), detect by shape and build tables.
        if csv::is_csv_metadata(map) {
            let mut flat: Vec<(String, String)> = map
                .iter()
                .filter(|(k, _)| !csv::MetadataArrayKey::all().contains_str(k))
                .map(|(k, v)| (format::format_key(k), format::format_value(v, k)))
                .collect();
            flat.sort_by(|a, b| a.0.cmp(&b.0));
            if !flat.is_empty() {
                sections.push(Section::KeyValue(KvSection {
                    title: Some(format::format_key(SectionKeys::CSV_METADATA)),
                    rows: flat,
                }));
            }
            sections.extend(csv::csv_metadata_to_sections(map));
            continue;
        }

        let mut flat = Vec::new();
        let mut nested = Vec::<(String, Map<String, Value>)>::new();
        let mut entries_here: Option<Vec<Value>> = None;
        let mut sheet_stats_here: Option<(String, Map<String, Value>)> = None;
        let mut common_pivots_here: Option<Vec<String>> = None;
        let mut csv_metadata_here: Option<(String, Map<String, Value>)> = None;
        let mut schema_here: Option<Value> = None;

        for (k, v) in map {
            if k == SectionKeys::ENTRIES {
                if let Some(arr) = v.as_array() {
                    entries_here = Some(arr.clone());
                }
                continue;
            }
            if k == SectionKeys::SCHEMA {
                schema_here = Some(v.clone());
                continue;
            }
            if k == SectionKeys::SHEET_STATS {
                if let Some(obj) = v.as_object()
                    && xlsx::is_sheet_stats(obj)
                {
                    sheet_stats_here = Some((k.clone(), obj.clone()));
                }
                if sheet_stats_here.is_none() {
                    if let Some(obj) = v.as_object() {
                        nested.push((k.clone(), obj.clone()));
                    } else {
                        flat.push((format::format_key(k), format::format_value(v, k)));
                    }
                }
                continue;
            }
            if k == SectionKeys::COMMON_PIVOTS {
                if let Some(arr) = v.as_array() {
                    let values: Vec<String> =
                        arr.iter().map(|v| format::format_value(v, k)).collect();
                    common_pivots_here = Some(values);
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
                continue;
            }
            if k == SectionKeys::CSV_METADATA {
                if let Some(obj) = v.as_object()
                    && csv::is_csv_metadata(obj)
                {
                    csv_metadata_here = Some((k.clone(), obj.clone()));
                }
                if csv_metadata_here.is_none() {
                    if let Some(obj) = v.as_object() {
                        nested.push((k.clone(), obj.clone()));
                    } else {
                        flat.push((format::format_key(k), format::format_value(v, k)));
                    }
                }
                continue;
            }
            match v {
                Value::Object(m) if !m.is_empty() => nested.push((k.clone(), m.clone())),
                _ => flat.push((format::format_key(k), format::format_value(v, k))),
            }
        }

        if !flat.is_empty() {
            sections.push(Section::KeyValue(KvSection {
                title: None,
                rows: flat,
            }));
        }

        if let Some(schema_val) = schema_here {
            let mut lines = schema_value_to_list(&schema_val);
            if lines.is_empty() {
                lines.push("—".to_string());
            }
            sections.push(Section::SingleColumnList(SingleColumnListSection {
                title: format::format_key(SectionKeys::SCHEMA),
                values: lines,
            }));
        }

        if let Some((section_key, obj)) = sheet_stats_here.take() {
            sections.push(xlsx::sheet_stats_to_section(&section_key, &obj));
        }

        if let Some(values) = common_pivots_here.take() {
            sections.push(Section::SingleColumnList(SingleColumnListSection {
                title: format::format_key(SectionKeys::COMMON_PIVOTS),
                values,
            }));
        }

        if let Some((section_key, csv_map)) = csv_metadata_here.take() {
            let mut flat_kv: Vec<(String, String)> = csv_map
                .iter()
                .filter(|(key, _)| !csv::MetadataArrayKey::all().contains_str(key))
                .map(|(key, val)| (format::format_key(key), format::format_value(val, key)))
                .collect();
            flat_kv.sort_by(|a, b| a.0.cmp(&b.0));
            if !flat_kv.is_empty() {
                sections.push(Section::KeyValue(KvSection {
                    title: Some(format::format_key(&section_key)),
                    rows: flat_kv,
                }));
            }
            sections.extend(csv::csv_metadata_to_sections(&csv_map));
        }

        for (section_key, map) in nested {
            let title = format::format_key(&section_key);
            let mut kv_rows = Vec::new();
            let mut entries_for_contents: Option<Vec<Value>> = None;
            let mut common_pivots_in_nested: Option<Vec<String>> = None;
            let mut schema_in_nested: Option<Value> = None;
            for (k, v) in &map {
                if k == SectionKeys::ENTRIES {
                    if let Some(arr) = v.as_array() {
                        entries_for_contents = Some(arr.clone());
                    }
                    continue;
                }
                if k == SectionKeys::SCHEMA {
                    schema_in_nested = Some(v.clone());
                    continue;
                }
                if k == SectionKeys::COMMON_PIVOTS {
                    if let Some(arr) = v.as_array() {
                        common_pivots_in_nested =
                            Some(arr.iter().map(|v| format::format_value(v, k)).collect());
                    } else {
                        kv_rows.push((format::format_key(k), format::format_value(v, k)));
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
            if let Some(schema_val) = schema_in_nested {
                let mut lines = schema_value_to_list(&schema_val);
                if lines.is_empty() {
                    lines.push("—".to_string());
                }
                sections.push(Section::SingleColumnList(SingleColumnListSection {
                    title: format::format_key(SectionKeys::SCHEMA),
                    values: lines,
                }));
            }
            if let Some(values) = common_pivots_in_nested {
                sections.push(Section::SingleColumnList(SingleColumnListSection {
                    title: format::format_key(SectionKeys::COMMON_PIVOTS),
                    values,
                }));
            }
            if let Some(arr) = entries_for_contents {
                push_contents_from_entries(&mut sections, arr);
            }
        }

        if let Some(arr) = entries_here {
            push_contents_from_entries(&mut sections, arr);
        }
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
