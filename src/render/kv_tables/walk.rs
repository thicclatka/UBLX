//! JSON map walk: root and nested object → sections (flat KV, schema, sheet_stats, common_pivots, csv_metadata, entries).

use serde_json::{Map, Value};

use crate::ui::UI_STRINGS;

use super::consts::SectionKeys;
use super::csv;
use super::format;
use super::schema;
use super::sections::{ContentsSection, KvSection, Section, SingleColumnListSection};
use super::xlsx;

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
                sub_title: false,
            }));
        }
    }
}

/// Walk root map once; push sections in order (flat KV, schema, sheet_stats, common_pivots, csv_metadata, then each nested, then entries). Uses JSON key names (SectionKeys) in the match.
pub fn push_root_parts(sections: &mut Vec<Section>, map: &Map<String, Value>) {
    let mut flat = Vec::new();
    let mut nested = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut sheet_stats = None;
    let mut common_pivots: Option<Vec<String>> = None;
    let mut csv_metadata = None;
    let mut tables_arr = None::<&Vec<Value>>;

    for (k, v) in map.iter() {
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
        push_contents_from_entries(sections, arr);
    }
}

/// Push one KV section per table object (e.g. sqlite_metadata.tables). Title from each object’s "name"; rows are all key-value pairs from that object (arrays/objects formatted for display).
const COLUMNS_KEY: &str = "columns";

fn push_tables_sections(sections: &mut Vec<Section>, arr: &[Value]) {
    for v in arr.iter().filter_map(Value::as_object) {
        let table_name = v
            .get("name")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| "Table".to_string());
        let rows: Vec<(String, String)> = v
            .iter()
            .filter(|(k, _)| k.as_str() != COLUMNS_KEY)
            .map(|(k, val)| (format::format_key(k), format::format_value(val, k)))
            .collect();
        if !rows.is_empty() {
            sections.push(Section::KeyValue(KvSection {
                title: Some(table_name.clone()),
                rows,
                sub_title: false,
            }));
        }
        if let Some(col_arr) = v.get(COLUMNS_KEY).and_then(Value::as_array) {
            let objs: Vec<&Map<String, Value>> = col_arr.iter().filter_map(|v| v.as_object()).collect();
            if let Some(first) = objs.first() {
                let column_keys: Vec<String> = first.keys().cloned().collect();
                let columns: Vec<String> = column_keys.iter().map(|k| format::format_key(k)).collect();
                let entries: Vec<Value> = col_arr.iter().filter(|v| v.is_object()).cloned().collect();
                if !entries.is_empty() {
                    sections.push(Section::Contents(ContentsSection {
                        title: format!("{} · Columns", table_name),
                        columns,
                        column_keys,
                        entries: entries.clone(),
                        sub_title: true,
                    }));
                    push_column_stats_sections(sections, &table_name, &entries);
                }
            }
        }
    }
}

/// Keys in a column object that hold stats (object → shown as "column_name · Stats type" KV sub-section).
const COLUMN_STATS_KEYS: &[&str] = &["text_stats", "boolean_stats", "numeric_stats", "date_stats"];

fn push_column_stats_sections(
    sections: &mut Vec<Section>,
    table_name: &str,
    column_objs: &[Value],
) {
    for col in column_objs.iter().filter_map(Value::as_object) {
        let col_name = col
            .get("name")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| "column".to_string());
        for &stats_key in COLUMN_STATS_KEYS {
            if let Some(stats_obj) = col.get(stats_key).and_then(Value::as_object) {
                let rows: Vec<(String, String)> = stats_obj
                    .iter()
                    .map(|(k, val)| (format::format_key(k), format::format_value(val, k)))
                    .collect();
                if !rows.is_empty() {
                    sections.push(Section::KeyValue(KvSection {
                        title: Some(format!(
                            "{} · {} · {}",
                            table_name,
                            col_name,
                            format::format_key(stats_key)
                        )),
                        rows,
                        sub_title: true,
                    }));
                }
            }
        }
    }
}

/// Walk nested map once (only entries, schema, common_pivots, tables are special; rest is flat KV). Push sections in order.
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

    for (k, v) in map.iter() {
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
        push_contents_from_entries(sections, arr);
    }
    if let Some(arr) = tables_arr {
        push_tables_sections(sections, arr);
    }
}
