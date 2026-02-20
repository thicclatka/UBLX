//! CSV metadata: one table per column type (string, date, boolean, etc.) from zahir csv_metadata.
//! Column keys and display labels come from the zahir JSON keys (e.g. column_names, null_percentages).

use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::format;
use super::sections::{ContentsSection, Section};

/// Keys in csv_metadata that are arrays (we build tables from them); scalars are shown as KV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataArrayKey {
    ColumnNames,
    ColumnTypes,
    NullPercentages,
    UniqueCounts,
    NumericStats,
    DateStats,
    BooleanStats,
}

impl MetadataArrayKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ColumnNames => "column_names",
            Self::ColumnTypes => "column_types",
            Self::NullPercentages => "null_percentages",
            Self::UniqueCounts => "unique_counts",
            Self::NumericStats => "numeric_stats",
            Self::DateStats => "date_stats",
            Self::BooleanStats => "boolean_stats",
        }
    }

    pub const fn all() -> &'static [MetadataArrayKey; 7] {
        &[
            Self::ColumnNames,
            Self::ColumnTypes,
            Self::NullPercentages,
            Self::UniqueCounts,
            Self::NumericStats,
            Self::DateStats,
            Self::BooleanStats,
        ]
    }
}

/// Extension so that `MetadataArrayKey::all().contains_str(k)` works.
pub trait MetadataArrayKeySliceExt {
    fn contains_str(&self, s: &str) -> bool;
}

impl MetadataArrayKeySliceExt for [MetadataArrayKey] {
    fn contains_str(&self, s: &str) -> bool {
        self.iter().any(|k| k.as_str() == s)
    }
}

/// Display label for a column key (overrides for brevity in tables).
fn csv_column_label(key: &str) -> String {
    match key {
        "unique_counts" => "Unique #".to_string(),
        "null_percentages" => "Null %".to_string(),
        "true_percentage" => "True %".to_string(),
        _ => format::format_key(key),
    }
}

fn csv_columns_display(column_keys: &[String]) -> Vec<String> {
    column_keys.iter().map(|k| csv_column_label(k)).collect()
}

/// True if `obj` looks like csv_metadata (has column_names and column_types arrays of same length).
pub fn is_csv_metadata(obj: &Map<String, Value>) -> bool {
    let names = obj.get("column_names").and_then(Value::as_array);
    let types = obj.get("column_types").and_then(Value::as_array);
    match (names, types) {
        (Some(n), Some(t)) => n.len() == t.len(),
        _ => false,
    }
}

/// Build one table per column type: "String columns", "Date columns", "Boolean columns", etc.
pub fn csv_metadata_to_sections(map: &Map<String, Value>) -> Vec<Section> {
    let names = match map.get("column_names").and_then(Value::as_array) {
        Some(a) => a,
        None => return vec![],
    };
    let types = match map.get("column_types").and_then(Value::as_array) {
        Some(a) => a,
        None => return vec![],
    };
    let n = names.len();
    if n != types.len() {
        return vec![];
    }
    let null_pct = map.get("null_percentages").and_then(Value::as_array);
    let unique = map.get("unique_counts").and_then(Value::as_array);
    let date_stats = map.get("date_stats").and_then(Value::as_array);
    let bool_stats = map.get("boolean_stats").and_then(Value::as_array);
    let num_stats = map.get("numeric_stats").and_then(Value::as_array);

    let mut by_type: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, t) in types.iter().enumerate() {
        let key = t.as_str().unwrap_or("unknown").to_string();
        by_type.entry(key).or_default().push(i);
    }

    let mut out = Vec::new();
    for (type_name, indices) in by_type {
        let section = match type_name.as_str() {
            "string" => table_string(names, null_pct, unique, indices),
            "date" => table_date(names, null_pct, unique, date_stats, indices),
            "boolean" => table_boolean(names, null_pct, unique, bool_stats, indices),
            _ => table_numeric_or_other(&type_name, names, null_pct, unique, num_stats, indices),
        };
        if let Some(s) = section {
            out.push(Section::Contents(s));
        }
    }
    out
}

fn row_common(
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    i: usize,
) -> Map<String, Value> {
    let mut row = Map::new();
    row.insert(
        "column_names".to_string(),
        names.get(i).cloned().unwrap_or(Value::Null),
    );
    row.insert(
        "null_percentages".to_string(),
        null_pct
            .and_then(|a| a.get(i).cloned())
            .unwrap_or(Value::Null),
    );
    row.insert(
        "unique_counts".to_string(),
        unique
            .and_then(|a| a.get(i).cloned())
            .unwrap_or(Value::Null),
    );
    row
}

fn table_string(
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    indices: Vec<usize>,
) -> Option<ContentsSection> {
    let column_keys = vec![
        "column_names".to_string(),
        "null_percentages".to_string(),
        "unique_counts".to_string(),
    ];
    let columns = csv_columns_display(&column_keys);
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| Value::Object(row_common(names, null_pct, unique, i)))
        .collect();
    if entries.is_empty() {
        return None;
    }
    Some(ContentsSection {
        title: "String columns".to_string(),
        columns,
        column_keys,
        entries,
    })
}

fn table_date(
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    date_stats: Option<&Vec<Value>>,
    indices: Vec<usize>,
) -> Option<ContentsSection> {
    let column_keys = vec![
        "column_names".to_string(),
        "null_percentages".to_string(),
        "unique_counts".to_string(),
        "span_days".to_string(),
        "min".to_string(),
        "max".to_string(),
    ];
    let columns = csv_columns_display(&column_keys);
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| {
            let mut row = row_common(names, null_pct, unique, i);
            let stats = date_stats.and_then(|a| a.get(i)).and_then(Value::as_object);
            if let Some(s) = stats {
                row.insert(
                    "span_days".to_string(),
                    s.get("span_days").cloned().unwrap_or(Value::Null),
                );
                row.insert(
                    "min".to_string(),
                    s.get("min").cloned().unwrap_or(Value::Null),
                );
                row.insert(
                    "max".to_string(),
                    s.get("max").cloned().unwrap_or(Value::Null),
                );
            } else {
                row.insert("span_days".to_string(), Value::Null);
                row.insert("min".to_string(), Value::Null);
                row.insert("max".to_string(), Value::Null);
            }
            Value::Object(row)
        })
        .collect();
    if entries.is_empty() {
        return None;
    }
    Some(ContentsSection {
        title: "Date columns".to_string(),
        columns,
        column_keys,
        entries,
    })
}

fn table_boolean(
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    bool_stats: Option<&Vec<Value>>,
    indices: Vec<usize>,
) -> Option<ContentsSection> {
    let column_keys = vec![
        "column_names".to_string(),
        "null_percentages".to_string(),
        "unique_counts".to_string(),
        "true_percentage".to_string(),
    ];
    let columns = csv_columns_display(&column_keys);
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| {
            let mut row = row_common(names, null_pct, unique, i);
            let pct = bool_stats
                .and_then(|a| a.get(i))
                .and_then(Value::as_object)
                .and_then(|o| o.get("true_percentage").cloned());
            row.insert("true_percentage".to_string(), pct.unwrap_or(Value::Null));
            Value::Object(row)
        })
        .collect();
    if entries.is_empty() {
        return None;
    }
    Some(ContentsSection {
        title: "Boolean columns".to_string(),
        columns,
        column_keys,
        entries,
    })
}

fn numeric_stats_keys(num_stats: Option<&Vec<Value>>) -> Vec<String> {
    let mut keys = BTreeMap::new();
    if let Some(arr) = num_stats {
        for v in arr.iter().filter_map(Value::as_object) {
            for k in v.keys() {
                keys.insert(k.clone(), ());
            }
        }
    }
    keys.into_keys().collect()
}

fn table_numeric_or_other(
    type_name: &str,
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    num_stats: Option<&Vec<Value>>,
    indices: Vec<usize>,
) -> Option<ContentsSection> {
    let stat_keys = numeric_stats_keys(num_stats);
    let mut column_keys = vec![
        "column_names".to_string(),
        "null_percentages".to_string(),
        "unique_counts".to_string(),
    ];
    column_keys.extend(stat_keys.clone());
    let columns = csv_columns_display(&column_keys);
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| {
            let mut row = row_common(names, null_pct, unique, i);
            let stats = num_stats.and_then(|a| a.get(i)).and_then(Value::as_object);
            for k in &stat_keys {
                let val = stats.and_then(|s| s.get(k).cloned()).unwrap_or(Value::Null);
                row.insert(k.clone(), val);
            }
            Value::Object(row)
        })
        .collect();
    if entries.is_empty() {
        return None;
    }
    let title = format!("{} columns", format::format_key(type_name));
    Some(ContentsSection {
        title,
        columns,
        column_keys,
        entries,
    })
}
