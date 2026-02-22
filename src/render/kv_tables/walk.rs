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

    for (k, v) in map.iter() {
        match k.as_str() {
            SectionKeys::ENTRIES => entries = v.as_array().cloned(),
            SectionKeys::SCHEMA => schema_val = Some(v.clone()),
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
    for (key, m) in nested {
        process_nested_map(sections, &key, &m);
    }
    if let Some(arr) = entries {
        push_contents_from_entries(sections, arr);
    }
}

/// Walk nested map once (only entries, schema, common_pivots are special; rest is flat KV). Push sections in order.
pub fn process_nested_map(
    sections: &mut Vec<Section>,
    section_key: &str,
    map: &Map<String, Value>,
) {
    let mut flat = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut common_pivots: Option<Vec<String>> = None;

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
            _ => flat.push((format::format_key(k), format::format_value(v, k))),
        }
    }

    let title = format::format_key(section_key);
    if !flat.is_empty() {
        sections.push(Section::KeyValue(KvSection {
            title: Some(title),
            rows: flat,
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
}
