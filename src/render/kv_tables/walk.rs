//! JSON map walk: root and nested object → sections (flat KV, schema, `sheet_stats`, `common_pivots`, `csv_metadata`, arrays of objects, `entries`).
//!
//! **Arrays of objects** (non-empty, every element is a JSON object), except [`SectionKeys::ENTRIES`],
//! are split into one KV table per element via [`push_tables_sections`] (title from `name`, else `path`, else `Table`).
//! Examples: `tables`, `npy_entries`, `datasets`, `variables`, `global_attributes` — no per-key wiring required.
//! This is driven by **JSON shape** in the stored `*_metadata` blob for any file type Zahir enriched, not by a fixed list of extensions.
//!
//! Flattening `NetCDF` `attributes` `[{name,value},…]` runs when the blob includes Zahir’s root
//! `file_type` (merged into each `*_metadata` object in [`crate::handlers::viewing::sectioned_preview_from_zahir`])
//! for [`WalkCtx`] only; that key is removed before building tables so it does not appear as a row.
//! Resolution uses [`crate::integrations::file_type_from_metadata_name`] → [`ZahirFT::NetCdf`].
//!
//! Compact **`columns`** stats (`name`, `t`, … per row) use [`column_metadata::push_column_metadata_sections`].
//! Stale parallel `column_names` / `column_types` JSON shows [`column_metadata::push_legacy_column_metadata_notice`]
//! instead of tables.

use serde_json::{Map, Value};
use std::collections::HashSet;

use crate::integrations::{ZahirFT, file_type_from_metadata_name};
use crate::ui::UI_STRINGS;

use super::column_metadata;
use super::consts::SectionKeys;
use super::format;
use super::schema;
use super::sections::{ContentsSection, KvSection, Section, SingleColumnListSection};
use super::xlsx;

/// JSON keys and display fallbacks shared by the metadata walk (Zahir shapes: `NetCDF`, HDF5, `SQLite`, …).
pub struct WalkKeyVars;

impl WalkKeyVars {
    pub const ATTRIBUTES: &'static str = "attributes";
    pub const NAME: &'static str = "name";
    pub const VALUE: &'static str = "value";
    pub const PATH: &'static str = "path";
    pub const COLUMNS: &'static str = "columns";
    pub const METADATA: &'static str = "_metadata";
    /// Root Zahir field merged into `*_metadata` for [`WalkCtx`] (stripped before KV display).
    pub const FILE_TYPE: &'static str = "file_type";
    pub const DEFAULT_TABLE_TITLE: &'static str = "Table";
    pub const DEFAULT_COLUMN_LABEL: &'static str = "column";
}

/// Format a map as KV rows (key/value pairs). Optionally exclude one key (e.g. [`WalkKeyVars::COLUMNS`]).
fn map_to_kv_rows(
    map_ref: &Map<String, Value>,
    exclude_key: Option<&str>,
) -> Vec<(String, String)> {
    map_ref
        .iter()
        .filter(|(k, _)| exclude_key != Some(k.as_str()))
        .map(|(k, val)| (format::format_key(k), format::format_value(val, k)))
        .collect()
}

/// Carries root Zahir `file_type` (merged into metadata JSON) for NetCDF-only attribute flattening.
#[derive(Clone, Copy, Default)]
pub struct WalkCtx {
    /// True when `file_type` is present and parses to [`ZahirFT::NetCdf`] via [`file_type_from_metadata_name`].
    is_netcdf: bool,
}

impl WalkCtx {
    #[must_use]
    pub fn from_root_map(map_ref: &Map<String, Value>) -> Self {
        let is_netcdf = map_ref
            .get(WalkKeyVars::FILE_TYPE)
            .and_then(|v| v.as_str())
            .and_then(file_type_from_metadata_name)
            .is_some_and(|ft| ft == ZahirFT::NetCdf);
        Self { is_netcdf }
    }
}

/// Drop [`WalkKeyVars::FILE_TYPE`] after [`WalkCtx::from_root_map`] so it is not rendered as metadata KV (category already shows type).
fn map_without_display_file_type(map_ref: &Map<String, Value>) -> Map<String, Value> {
    let mut m = map_ref.clone();
    m.remove(WalkKeyVars::FILE_TYPE);
    m
}

fn flatten_name_value_attribute_rows(arr_ref: &[Value]) -> Option<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(arr_ref.len());
    for v in arr_ref {
        let obj = v.as_object()?;
        let name = obj.get(WalkKeyVars::NAME)?.as_str()?;
        let value = obj.get(WalkKeyVars::VALUE)?;
        // Keep identifier as-is (e.g. `_FillValue`); do not title-case via [`format::format_key`].
        out.push((name.to_string(), format::format_value(value, name)));
    }
    Some(out)
}

/// Like [`map_to_kv_rows`], but expands `attributes` name/value lists into flat rows for the same section.
fn map_to_kv_rows_flat_name_value_attributes(
    map_ref: &Map<String, Value>,
    exclude_key: Option<&str>,
) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (k, val) in map_ref {
        if exclude_key == Some(k.as_str()) {
            continue;
        }
        if k == WalkKeyVars::ATTRIBUTES
            && let Some(arr) = val.as_array()
            && let Some(flat) = flatten_name_value_attribute_rows(arr)
        {
            rows.extend(flat);
            continue;
        }
        rows.push((format::format_key(k), format::format_value(val, k)));
    }
    rows
}

/// From an array of JSON objects, get column keys (from all objects, first object's order then any extra keys), display column names, and entries. Returns None if empty or no objects.
fn object_array_to_contents_data(
    arr_ref: &[Value],
) -> Option<(Vec<String>, Vec<String>, Vec<Value>)> {
    let objs: Vec<&Map<String, Value>> = arr_ref.iter().filter_map(Value::as_object).collect();
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
    let entries: Vec<Value> = arr_ref.iter().filter(|v| v.is_object()).cloned().collect();
    if entries.is_empty() {
        return None;
    }
    Some((column_keys, columns, entries))
}

/// True when `arr` is non-empty and every item is a JSON object (uniform record list).
fn is_uniform_object_array(arr_ref: &[Value]) -> bool {
    !arr_ref.is_empty() && arr_ref.iter().all(Value::is_object)
}

/// `entries` builds one multi-column Contents table; other uniform object arrays become separate KV tables per row.
#[inline]
fn array_is_record_table_list(key_ref: &str, arr_ref: &[Value]) -> bool {
    key_ref != SectionKeys::ENTRIES && is_uniform_object_array(arr_ref)
}

fn push_contents_from_entries(sections_mut_ref: &mut Vec<Section>, arr_ref: &[Value]) {
    if let Some((column_keys, columns, entries)) = object_array_to_contents_data(arr_ref) {
        sections_mut_ref.push(Section::Contents(ContentsSection {
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
pub fn root_parts_sections(map_ref: &Map<String, Value>) -> Vec<Section> {
    let mut sections = Vec::new();
    push_root_parts(&mut sections, map_ref);
    sections
}

/// Walk root map once; push sections in order (flat KV, schema, `sheet_stats`, `common_pivots`, `csv_metadata`, uniform object arrays, then each nested, then entries). Uses JSON key names (`SectionKeys`) in the match.
pub fn push_root_parts(sections_mut_ref: &mut Vec<Section>, map_ref: &Map<String, Value>) {
    let ctx = WalkCtx::from_root_map(map_ref);
    let map_owned = map_without_display_file_type(map_ref);
    push_root_parts_inner(sections_mut_ref, &map_owned, ctx);
}

fn push_root_parts_inner(
    sections_mut_ref: &mut Vec<Section>,
    map_ref: &Map<String, Value>,
    ctx: WalkCtx,
) {
    let mut flat = Vec::new();
    let mut nested = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut sheet_stats = None;
    let mut common_pivots: Option<Vec<String>> = None;
    let mut column_metadata_compact: Option<(String, Map<String, Value>)> = None;
    let mut column_metadata_legacy_title: Option<String> = None;
    let mut record_object_arrays: Vec<&Vec<Value>> = Vec::new();

    for (k, v) in map_ref {
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
                    if column_metadata::is_compact_column_metadata(obj) {
                        column_metadata_compact = Some((k.clone(), obj.clone()));
                    } else if column_metadata::is_legacy_parallel_column_metadata(obj) {
                        column_metadata_legacy_title = Some(format::format_key(k));
                    } else {
                        nested.push((k.clone(), obj.clone()));
                    }
                } else {
                    flat.push((format::format_key(k), format::format_value(v, k)));
                }
            }
            _ => match v {
                Value::Array(arr) if array_is_record_table_list(k.as_str(), arr) => {
                    record_object_arrays.push(arr);
                }
                Value::Object(m) if !m.is_empty() => nested.push((k.clone(), m.clone())),
                _ => flat.push((format::format_key(k), format::format_value(v, k))),
            },
        }
    }

    if !flat.is_empty() {
        sections_mut_ref.push(Section::KeyValue(KvSection {
            title: None,
            rows: flat,
            sub_title: false,
        }));
    }
    if let Some(v) = schema_val {
        schema::push_schema_section(sections_mut_ref, &v);
    }
    if let Some((key, obj)) = sheet_stats {
        sections_mut_ref.push(xlsx::sheet_stats_to_section(&key, &obj));
    }
    if let Some(values) = common_pivots.filter(|pivots| !pivots.is_empty()) {
        sections_mut_ref.push(Section::SingleColumnList(SingleColumnListSection {
            title: format::format_key(SectionKeys::COMMON_PIVOTS),
            values,
        }));
    }
    if let Some((key, meta)) = column_metadata_compact {
        column_metadata::push_column_metadata_sections(sections_mut_ref, &key, &meta);
    }
    if let Some(title) = column_metadata_legacy_title {
        column_metadata::push_legacy_column_metadata_notice(sections_mut_ref, Some(title), false);
    }
    for arr in record_object_arrays {
        push_tables_sections(sections_mut_ref, arr, ctx);
    }
    for (key, m) in nested {
        process_nested_map(sections_mut_ref, &key, &m, &ctx);
    }
    if let Some(arr) = entries {
        push_contents_from_entries(sections_mut_ref, &arr);
    }
}

#[inline]
fn object_name_or(obj_ref: &Map<String, Value>, default_ref: &str) -> String {
    obj_ref
        .get(WalkKeyVars::NAME)
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| {
            obj_ref
                .get(WalkKeyVars::PATH)
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| default_ref.to_string())
}

fn push_tables_sections(sections_mut_ref: &mut Vec<Section>, arr_ref: &[Value], walk_ctx: WalkCtx) {
    for v in arr_ref.iter().filter_map(Value::as_object) {
        let table_name = object_name_or(v, WalkKeyVars::DEFAULT_TABLE_TITLE);
        if column_metadata::is_compact_column_metadata(v) {
            column_metadata::push_column_metadata_sections(
                sections_mut_ref,
                table_name.as_str(),
                v,
            );
            continue;
        }
        if column_metadata::is_legacy_parallel_column_metadata(v) {
            column_metadata::push_legacy_column_metadata_notice(
                sections_mut_ref,
                Some(table_name.clone()),
                true,
            );
            continue;
        }
        let rows = if walk_ctx.is_netcdf {
            map_to_kv_rows_flat_name_value_attributes(v, Some(WalkKeyVars::COLUMNS))
        } else {
            map_to_kv_rows(v, Some(WalkKeyVars::COLUMNS))
        };
        if !rows.is_empty() {
            sections_mut_ref.push(Section::KeyValue(KvSection {
                title: Some(table_name.clone()),
                rows,
                sub_title: false,
            }));
        }
        if let Some(col_arr) = v.get(WalkKeyVars::COLUMNS).and_then(Value::as_array) {
            if column_metadata::is_compact_column_stats_array(col_arr) {
                sections_mut_ref.extend(column_metadata::typed_sections_from_compact_columns(
                    col_arr,
                    Some(table_name.as_str()),
                ));
            } else if let Some((column_keys, columns, entries)) =
                object_array_to_contents_data(col_arr)
            {
                sections_mut_ref.push(Section::Contents(ContentsSection {
                    title: format::join_dot([&table_name, UI_STRINGS.tables.columns_title]),
                    columns,
                    column_keys,
                    entries: entries.clone(),
                    sub_title: true,
                }));
                push_column_stats_sections(sections_mut_ref, &table_name, &entries);
            }
        }
    }
}

fn push_column_stats_sections(
    sections_mut_ref: &mut Vec<Section>,
    table_name_ref: &str,
    column_objs_ref: &[Value],
) {
    for col in column_objs_ref.iter().filter_map(Value::as_object) {
        let col_name = object_name_or(col, WalkKeyVars::DEFAULT_COLUMN_LABEL);
        for (stats_key, stats_val) in col {
            if stats_key == WalkKeyVars::NAME {
                continue;
            }
            if let Some(stats_obj) = stats_val.as_object() {
                let rows = map_to_kv_rows(stats_obj, None);
                if !rows.is_empty() {
                    let stats_label = format::format_key(stats_key);
                    sections_mut_ref.push(Section::KeyValue(KvSection {
                        title: Some(format::join_dot([
                            table_name_ref,
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

/// Walk nested map once (entries, schema, `common_pivots`, uniform object arrays; rest flat KV). Push sections in order.
pub fn process_nested_map(
    sections_mut_ref: &mut Vec<Section>,
    section_key_ref: &str,
    map_ref: &Map<String, Value>,
    ctx_ref: &WalkCtx,
) {
    if column_metadata::is_compact_column_metadata(map_ref) {
        column_metadata::push_column_metadata_sections(sections_mut_ref, section_key_ref, map_ref);
        return;
    }
    if column_metadata::is_legacy_parallel_column_metadata(map_ref) {
        column_metadata::push_legacy_column_metadata_notice(
            sections_mut_ref,
            Some(format::format_key(section_key_ref)),
            false,
        );
        return;
    }

    let mut flat = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut common_pivots: Option<Vec<String>> = None;
    let mut record_object_arrays: Vec<&Vec<Value>> = Vec::new();

    for (k, v) in map_ref {
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
            _ => match v {
                Value::Array(arr) if array_is_record_table_list(k.as_str(), arr) => {
                    record_object_arrays.push(arr);
                }
                _ => flat.push((format::format_key(k), format::format_value(v, k))),
            },
        }
    }

    let title = format::format_key(section_key_ref);
    if !flat.is_empty() {
        sections_mut_ref.push(Section::KeyValue(KvSection {
            title: Some(title),
            rows: flat,
            sub_title: false,
        }));
    }
    if let Some(v) = schema_val {
        schema::push_schema_section(sections_mut_ref, &v);
    }
    if let Some(values) = common_pivots.filter(|pivots| !pivots.is_empty()) {
        sections_mut_ref.push(Section::SingleColumnList(SingleColumnListSection {
            title: format::format_key(SectionKeys::COMMON_PIVOTS),
            values,
        }));
    }
    if let Some(arr) = entries {
        push_contents_from_entries(sections_mut_ref, &arr);
    }
    for arr in record_object_arrays {
        push_tables_sections(sections_mut_ref, arr, *ctx_ref);
    }
}
