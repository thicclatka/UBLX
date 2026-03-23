//! JSON map walk: root and nested object → sections (flat KV, schema, `sheet_stats`, `common_pivots`, `csv_metadata`, entries).

use serde_json::{Map, Value};
use std::collections::HashSet;

use crate::ui::UI_STRINGS;

use super::consts::SectionKeys;
use super::csv;
use super::format;
use super::schema;
use super::sections::{ContentsSection, KvSection, Section, SingleColumnListSection};
use super::xlsx;

/// Format a map as KV rows (key/value pairs). Optionally exclude one key (e.g. "columns").
fn map_to_kv_rows(map: &Map<String, Value>, exclude_key: Option<&str>) -> Vec<(String, String)> {
    map.iter()
        .filter(|(k, _)| exclude_key != Some(k.as_str()))
        .map(|(k, val)| (format::format_key(k), format::format_value(val, k)))
        .collect()
}

/// From an array of JSON objects, get column keys (from all objects, first object's order then any extra keys), display column names, and entries. Returns None if empty or no objects.
fn object_array_to_contents_data(arr: &[Value]) -> Option<(Vec<String>, Vec<String>, Vec<Value>)> {
    let objs: Vec<&Map<String, Value>> = arr.iter().filter_map(Value::as_object).collect();
    let first = objs.first()?;
    let mut column_keys: Vec<String> = first.keys().cloned().collect();
    let mut seen: HashSet<String> = column_keys.iter().cloned().collect();
    for obj in objs.iter().skip(1) {
        for k in obj.keys() {
            if seen.insert(k.clone()) {
                column_keys.push(k.clone());
            }
        }
    }
    let columns: Vec<String> = column_keys.iter().map(|k| format::format_key(k)).collect();
    let entries: Vec<Value> = arr.iter().filter(|v| v.is_object()).cloned().collect();
    if entries.is_empty() {
        return None;
    }
    Some((column_keys, columns, entries))
}

fn push_contents_from_entries(sections: &mut Vec<Section>, arr: &[Value]) {
    if let Some((column_keys, columns, entries)) = object_array_to_contents_data(arr) {
        sections.push(Section::Contents(ContentsSection {
            title: UI_STRINGS.tables.contents_title.to_string(),
            columns,
            column_keys,
            entries,
            sub_title: false,
        }));
    }
}

/// Same as [`push_root_parts`] but returns a new vec. Used when parsing blobs in parallel.
#[must_use]
pub fn root_parts_sections(map: &Map<String, Value>) -> Vec<Section> {
    let mut sections = Vec::new();
    push_root_parts(&mut sections, map);
    sections
}

/// Walk root map once; push sections in order (flat KV, schema, `sheet_stats`, `common_pivots`, `csv_metadata`, then each nested, then entries). Uses JSON key names (`SectionKeys`) in the match.
pub fn push_root_parts(sections: &mut Vec<Section>, map: &Map<String, Value>) {
    let mut flat = Vec::new();
    let mut nested = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut sheet_stats = None;
    let mut common_pivots: Option<Vec<String>> = None;
    let mut csv_metadata = None;
    let mut tables_arr = None::<&Vec<Value>>;

    for (k, v) in map {
        match k.as_str() {
            SectionKeys::ENTRIES => entries = v.as_array().cloned(),
            SectionKeys::SCHEMA => schema_val = Some(v.clone()),
            SectionKeys::TABLES => {
                if let Some(arr) = v.as_array() {
                    tables_arr = Some(arr);
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            SectionKeys::SHEET_STATS => {
                if let Some(obj) = v.as_object() {
                    if xlsx::is_sheet_stats(obj) {
                        sheet_stats = Some((k.clone(), obj.clone()));
                    } else {
                        nested.push((k.clone(), obj.clone()));
                    }
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            SectionKeys::COMMON_PIVOTS => {
                if let Some(arr) = v.as_array() {
                    common_pivots =
                        Some(arr.iter().map(|val| format::format_value(val, k)).collect());
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            SectionKeys::CSV_METADATA => {
                if let Some(obj) = v.as_object() {
                    if csv::is_csv_metadata(obj) {
                        csv_metadata = Some((k.clone(), obj.clone()));
                    } else {
                        nested.push((k.clone(), obj.clone()));
                    }
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            _ => match v {
                Value::Object(m) if !m.is_empty() => nested.push((k.clone(), m.clone())),
                _ => flat.push((format::format_key(k), format::format_value(v, k))),
            },
        }
    }

    if !flat.is_empty() {
        sections.push(Section::KeyValue(KvSection {
            title: None,
            rows: flat,
            sub_title: false,
        }));
    }
    if let Some(v) = schema_val {
        schema::push_schema_section(sections, &v);
    }
    if let Some((key, obj)) = sheet_stats {
        sections.push(xlsx::sheet_stats_to_section(&key, &obj));
    }
    if let Some(values) = common_pivots.filter(|v| !v.is_empty()) {
        sections.push(Section::SingleColumnList(SingleColumnListSection {
            title: format::format_key(SectionKeys::COMMON_PIVOTS),
            values,
        }));
    }
    if let Some((key, csv_map)) = csv_metadata {
        csv::push_csv_metadata_sections(sections, &key, &csv_map);
    }
    if let Some(arr) = tables_arr {
        push_tables_sections(sections, arr);
    }
    for (key, m) in nested {
        process_nested_map(sections, &key, &m);
    }
    if let Some(arr) = entries {
        push_contents_from_entries(sections, &arr);
    }
}

/// Push one KV section per table object (e.g. `sqlite_metadata.tables`). Title from each object’s `"name"`; rows are all key-value pairs from that object (arrays/objects formatted for display).
const COLUMNS_KEY: &str = "columns";

fn push_tables_sections(sections: &mut Vec<Section>, arr: &[Value]) {
    for v in arr.iter().filter_map(Value::as_object) {
        let table_name = v
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| "Table".to_string(), String::from);
        let rows = map_to_kv_rows(v, Some(COLUMNS_KEY));
        if !rows.is_empty() {
            sections.push(Section::KeyValue(KvSection {
                title: Some(table_name.clone()),
                rows,
                sub_title: false,
            }));
        }
        if let Some(col_arr) = v.get(COLUMNS_KEY).and_then(Value::as_array)
            && let Some((column_keys, columns, entries)) = object_array_to_contents_data(col_arr)
        {
            sections.push(Section::Contents(ContentsSection {
                title: format::join_dot([&table_name, UI_STRINGS.tables.columns_title]),
                columns,
                column_keys,
                entries: entries.clone(),
                sub_title: true,
            }));
            push_column_stats_sections(sections, &table_name, &entries);
        }
    }
}

fn push_column_stats_sections(
    sections: &mut Vec<Section>,
    table_name: &str,
    column_objs: &[Value],
) {
    for col in column_objs.iter().filter_map(Value::as_object) {
        let col_name = col
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| "column".to_string(), String::from);
        for (stats_key, stats_val) in col {
            if stats_key == "name" {
                continue;
            }
            if let Some(stats_obj) = stats_val.as_object() {
                let rows = map_to_kv_rows(stats_obj, None);
                if !rows.is_empty() {
                    let stats_label = format::format_key(stats_key);
                    sections.push(Section::KeyValue(KvSection {
                        title: Some(format::join_dot([
                            table_name,
                            col_name.as_str(),
                            stats_label.as_str(),
                        ])),
                        rows,
                        sub_title: true,
                    }));
                }
            }
        }
    }
}

/// Walk nested map once (only entries, schema, `common_pivots`, tables are special; rest is flat KV). Push sections in order.
pub fn process_nested_map(
    sections: &mut Vec<Section>,
    section_key: &str,
    map: &Map<String, Value>,
) {
    let mut flat = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut common_pivots: Option<Vec<String>> = None;
    let mut tables_arr = None::<&Vec<Value>>;

    for (k, v) in map {
        match k.as_str() {
            SectionKeys::ENTRIES => entries = v.as_array().cloned(),
            SectionKeys::SCHEMA => schema_val = Some(v.clone()),
            SectionKeys::COMMON_PIVOTS => {
                if let Some(arr) = v.as_array() {
                    common_pivots =
                        Some(arr.iter().map(|val| format::format_value(val, k)).collect());
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            SectionKeys::TABLES => {
                if v.is_array() {
                    tables_arr = v.as_array();
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            _ => flat.push((format::format_key(k), format::format_value(v, k))),
        }
    }

    let title = format::format_key(section_key);
    if !flat.is_empty() {
        sections.push(Section::KeyValue(KvSection {
            title: Some(title),
            rows: flat,
            sub_title: false,
        }));
    }
    if let Some(v) = schema_val {
        schema::push_schema_section(sections, &v);
    }
    if let Some(values) = common_pivots.filter(|v| !v.is_empty()) {
        sections.push(Section::SingleColumnList(SingleColumnListSection {
            title: format::format_key(SectionKeys::COMMON_PIVOTS),
            values,
        }));
    }
    if let Some(arr) = entries {
        push_contents_from_entries(sections, &arr);
    }
    if let Some(arr) = tables_arr {
        push_tables_sections(sections, arr);
    }
}
