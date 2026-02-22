//! CSV metadata: one table per column type (string, date, boolean, etc.) from zahir csv_metadata.
//! Column keys and display labels come from the zahir JSON keys (e.g. column_names, null_percentages).

use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};

use super::consts::SectionKeys;
use super::format;
use super::sections::{ContentsSection, KvSection, Section};

/// Keys inside date_stats objects (span_days, min, max).
struct DateStatsKeys;
impl DateStatsKeys {
    const SPAN_DAYS: &'static str = "span_days";
    const MIN: &'static str = "min";
    const MAX: &'static str = "max";
}

/// Key inside boolean_stats objects for true percentage.
const BOOLEAN_STATS_TRUE_PCT: &str = "true_percentage";

/// Column type from csv_metadata column_types array; drives which table/section we build.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColumnType {
    String,
    Date,
    Boolean,
    Other,
}

impl ColumnType {
    fn from_type_str(s: &str) -> Self {
        match s {
            "string" => Self::String,
            "date" => Self::Date,
            "boolean" => Self::Boolean,
            _ => Self::Other,
        }
    }

    fn section_title(self, type_name: &str) -> String {
        match self {
            Self::String => "String columns".to_string(),
            Self::Date => "Date columns".to_string(),
            Self::Boolean => "Boolean columns".to_string(),
            Self::Other => format!("{} columns", format::format_key(type_name)),
        }
    }
}

const TYPE_UNKNOWN: &str = "unknown";

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
        k if k == MetadataArrayKey::UniqueCounts.as_str() => "Unique #".to_string(),
        k if k == MetadataArrayKey::NullPercentages.as_str() => "Null %".to_string(),
        k if k == BOOLEAN_STATS_TRUE_PCT => "True %".to_string(),
        _ => format::format_key(key),
    }
}

fn csv_columns_display(column_keys: &[String]) -> Vec<String> {
    column_keys.iter().map(|k| csv_column_label(k)).collect()
}

fn contents_section(
    title: String,
    column_keys: Vec<String>,
    entries: Vec<Value>,
) -> Option<ContentsSection> {
    if entries.is_empty() {
        return None;
    }
    Some(ContentsSection {
        title,
        columns: csv_columns_display(&column_keys),
        column_keys,
        entries,
        sub_title: false,
    })
}

/// True if `obj` looks like csv_metadata (has column_names and column_types arrays of same length).
pub fn is_csv_metadata(obj: &Map<String, Value>) -> bool {
    let names = obj
        .get(MetadataArrayKey::ColumnNames.as_str())
        .and_then(Value::as_array);
    let types = obj
        .get(MetadataArrayKey::ColumnTypes.as_str())
        .and_then(Value::as_array);
    match (names, types) {
        (Some(n), Some(t)) => n.len() == t.len(),
        _ => false,
    }
}

/// Build one table per column type: "String columns", "Date columns", "Boolean columns", etc.
pub fn csv_metadata_to_sections(map: &Map<String, Value>) -> Vec<Section> {
    let names = match map
        .get(MetadataArrayKey::ColumnNames.as_str())
        .and_then(Value::as_array)
    {
        Some(a) => a,
        None => return vec![],
    };
    let types = match map
        .get(MetadataArrayKey::ColumnTypes.as_str())
        .and_then(Value::as_array)
    {
        Some(a) => a,
        None => return vec![],
    };
    let n = names.len();
    if n != types.len() {
        return vec![];
    }
    let null_pct = map
        .get(MetadataArrayKey::NullPercentages.as_str())
        .and_then(Value::as_array);
    let unique = map
        .get(MetadataArrayKey::UniqueCounts.as_str())
        .and_then(Value::as_array);
    let date_stats = map
        .get(MetadataArrayKey::DateStats.as_str())
        .and_then(Value::as_array);
    let bool_stats = map
        .get(MetadataArrayKey::BooleanStats.as_str())
        .and_then(Value::as_array);
    let num_stats = map
        .get(MetadataArrayKey::NumericStats.as_str())
        .and_then(Value::as_array);

    let mut by_type: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, t) in types.iter().enumerate() {
        let key = t.as_str().unwrap_or(TYPE_UNKNOWN).to_string();
        by_type.entry(key).or_default().push(i);
    }

    let mut out = Vec::new();
    for (type_name, indices) in by_type {
        let col_type = ColumnType::from_type_str(&type_name);
        let section = match col_type {
            ColumnType::String => table_string(names, null_pct, unique, indices),
            ColumnType::Date => table_date(names, null_pct, unique, date_stats, indices),
            ColumnType::Boolean => table_boolean(names, null_pct, unique, bool_stats, indices),
            ColumnType::Other => {
                table_numeric_or_other(&type_name, names, null_pct, unique, num_stats, indices)
            }
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
        MetadataArrayKey::ColumnNames.as_str().to_string(),
        names.get(i).cloned().unwrap_or(Value::Null),
    );
    row.insert(
        MetadataArrayKey::NullPercentages.as_str().to_string(),
        null_pct
            .and_then(|a| a.get(i).cloned())
            .unwrap_or(Value::Null),
    );
    row.insert(
        MetadataArrayKey::UniqueCounts.as_str().to_string(),
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
    let column_keys = common_column_keys();
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| Value::Object(row_common(names, null_pct, unique, i)))
        .collect();
    contents_section(
        ColumnType::String.section_title("string"),
        column_keys,
        entries,
    )
}

fn common_column_keys() -> Vec<String> {
    vec![
        MetadataArrayKey::ColumnNames.as_str().to_string(),
        MetadataArrayKey::NullPercentages.as_str().to_string(),
        MetadataArrayKey::UniqueCounts.as_str().to_string(),
    ]
}

fn table_date(
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    date_stats: Option<&Vec<Value>>,
    indices: Vec<usize>,
) -> Option<ContentsSection> {
    let mut column_keys = common_column_keys();
    column_keys.extend([
        DateStatsKeys::SPAN_DAYS.to_string(),
        DateStatsKeys::MIN.to_string(),
        DateStatsKeys::MAX.to_string(),
    ]);
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| {
            let mut row = row_common(names, null_pct, unique, i);
            let stats = date_stats.and_then(|a| a.get(i)).and_then(Value::as_object);
            if let Some(s) = stats {
                row.insert(
                    DateStatsKeys::SPAN_DAYS.to_string(),
                    s.get(DateStatsKeys::SPAN_DAYS)
                        .cloned()
                        .unwrap_or(Value::Null),
                );
                row.insert(
                    DateStatsKeys::MIN.to_string(),
                    s.get(DateStatsKeys::MIN).cloned().unwrap_or(Value::Null),
                );
                row.insert(
                    DateStatsKeys::MAX.to_string(),
                    s.get(DateStatsKeys::MAX).cloned().unwrap_or(Value::Null),
                );
            } else {
                row.insert(DateStatsKeys::SPAN_DAYS.to_string(), Value::Null);
                row.insert(DateStatsKeys::MIN.to_string(), Value::Null);
                row.insert(DateStatsKeys::MAX.to_string(), Value::Null);
            }
            Value::Object(row)
        })
        .collect();
    contents_section(ColumnType::Date.section_title("date"), column_keys, entries)
}

fn table_boolean(
    names: &[Value],
    null_pct: Option<&Vec<Value>>,
    unique: Option<&Vec<Value>>,
    bool_stats: Option<&Vec<Value>>,
    indices: Vec<usize>,
) -> Option<ContentsSection> {
    let mut column_keys = common_column_keys();
    column_keys.push(BOOLEAN_STATS_TRUE_PCT.to_string());
    let entries: Vec<Value> = indices
        .into_iter()
        .map(|i| {
            let mut row = row_common(names, null_pct, unique, i);
            let pct = bool_stats
                .and_then(|a| a.get(i))
                .and_then(Value::as_object)
                .and_then(|o| o.get(BOOLEAN_STATS_TRUE_PCT).cloned());
            row.insert(
                BOOLEAN_STATS_TRUE_PCT.to_string(),
                pct.unwrap_or(Value::Null),
            );
            Value::Object(row)
        })
        .collect();
    contents_section(
        ColumnType::Boolean.section_title("boolean"),
        column_keys,
        entries,
    )
}

/// Collect numeric_stats keys in JSON order: first object’s keys in order, then any keys from other objects not yet seen.
fn numeric_stats_keys(num_stats: Option<&Vec<Value>>) -> Vec<String> {
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    if let Some(arr) = num_stats {
        for v in arr.iter().filter_map(Value::as_object) {
            for k in v.keys() {
                if seen.insert(k.clone()) {
                    order.push(k.clone());
                }
            }
        }
    }
    order
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
    let mut column_keys = common_column_keys();
    column_keys.extend(stat_keys.clone());
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
    contents_section(
        ColumnType::Other.section_title(type_name),
        column_keys,
        entries,
    )
}

fn csv_flat_kv(csv_map: &Map<String, Value>) -> Vec<(String, String)> {
    csv_map
        .iter()
        .filter(|(key, _)| !MetadataArrayKey::all().contains_str(key))
        .map(|(key, val)| (format::format_key(key), format::format_value(val, key)))
        .collect()
}

/// Push flat KV section (if any) then all array-based tables. Shared by root CSV blob and nested csv_metadata.
fn push_csv_flat_and_tables(
    sections: &mut Vec<Section>,
    title: Option<String>,
    csv_map: &Map<String, Value>,
) {
    let flat_kv = csv_flat_kv(csv_map);
    if !flat_kv.is_empty() {
        sections.push(Section::KeyValue(KvSection {
            title,
            rows: flat_kv,
            sub_title: false,
        }));
    }
    sections.extend(csv_metadata_to_sections(csv_map));
}

/// Push flat KV section for csv_metadata scalars, then all array-based tables from `csv_metadata_to_sections`.
pub fn push_csv_metadata_sections(
    sections: &mut Vec<Section>,
    section_key: &str,
    csv_map: &Map<String, Value>,
) {
    push_csv_flat_and_tables(sections, Some(format::format_key(section_key)), csv_map);
}

/// Build sections when the root object is CSV metadata (one KV table + array tables).
pub fn sections_from_csv_root(map: &Map<String, Value>) -> Vec<Section> {
    let mut out = Vec::new();
    push_csv_flat_and_tables(
        &mut out,
        Some(format::format_key(SectionKeys::CSV_METADATA)),
        map,
    );
    out
}
