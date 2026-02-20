//! XLSX-specific metadata: table built from a key (e.g. sheet_stats) and nested objects (sheet name -> { rows, columns, ... }).

use serde_json::{Map, Value};

use super::format;
use super::sections::{ContentsSection, Section};

const COL_NAME: &str = "Name";

/// If `obj` is a sheet_stats-style object (name -> { key: value, ... }), returns a Section. Title from `section_key` (e.g. "sheet_stats" -> "Sheet Stats"); column keys are COL_NAME plus the keys from the first nested object (e.g. "rows", "columns").
pub fn sheet_stats_to_section(section_key: &str, obj: &Map<String, Value>) -> Section {
    let data_keys: Vec<String> = obj
        .values()
        .find_map(|v| v.as_object())
        .map(|o| {
            let mut k: Vec<String> = o.keys().cloned().collect();
            k.sort();
            k
        })
        .unwrap_or_default();
    let column_keys: Vec<String> = std::iter::once(COL_NAME.to_string())
        .chain(data_keys.clone())
        .collect();
    let columns: Vec<String> = column_keys.iter().map(|k| format::format_key(k)).collect();

    let mut entries: Vec<Value> = Vec::new();
    for (name, v) in obj {
        let sub = match v.as_object() {
            Some(o) => o,
            None => continue,
        };
        let mut row = serde_json::Map::new();
        row.insert(COL_NAME.to_string(), Value::String(name.clone()));
        for key in &data_keys {
            let val = sub.get(key).cloned().unwrap_or(Value::Null);
            row.insert(key.clone(), val);
        }
        entries.push(Value::Object(row));
    }
    entries.sort_by(|a, b| {
        let na = a.get(COL_NAME).and_then(Value::as_str).unwrap_or("");
        let nb = b.get(COL_NAME).and_then(Value::as_str).unwrap_or("");
        na.cmp(nb)
    });
    Section::Contents(ContentsSection {
        title: format::format_key(section_key),
        columns,
        column_keys,
        entries,
    })
}

/// Returns true if `obj` looks like sheet_stats (object with at least one value that has "rows" and "columns").
pub fn is_sheet_stats(obj: &Map<String, Value>) -> bool {
    let mut it = obj.values();
    let first = match it.next() {
        Some(v) => v,
        None => return false,
    };
    let sub = match first.as_object() {
        Some(o) => o,
        None => return false,
    };
    sub.contains_key("rows") && sub.contains_key("columns")
}
