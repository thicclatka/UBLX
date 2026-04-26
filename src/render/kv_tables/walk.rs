//! JSON map walk: root and nested object → sections (flat KV, schema, `sheet_stats`, `common_pivots`, `csv_metadata`, arrays of objects, `entries`).
//!
//! Nested object sections and values spilled from [`SectionKeys::ENTRIES`] rows use a compound display title: parent label `·` JSON key
//! (e.g. a row `name` + `struct_subtree` → `BinnedLicks · Struct Subtree`). Empty `[]` in a row is not spilled; uniform object
//! arrays in a row use the same per-record rules as top-level list arrays, with the row’s label as a title prefix.
//!
//! **Arrays of objects** (non-empty, every element is a JSON object), except [`SectionKeys::ENTRIES`],
//! are split into one KV table per element via [`push_tables_sections`] (title from `name`, else `path`,
//! else the array’s JSON key formatted plus `·` and the 0-based index, else `Table`).
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
    max_array_inline: usize,
) -> Vec<(String, String)> {
    map_ref
        .iter()
        .filter(|(k, _)| exclude_key != Some(k.as_str()))
        .map(|(k, val)| {
            (
                format::format_key(k),
                format::format_value(val, k, max_array_inline),
            )
        })
        .collect()
}

/// Carries root Zahir `file_type` and how many array primitives to show inline in key/value values.
#[derive(Clone, Copy)]
pub struct WalkCtx {
    /// True when `file_type` is present and parses to [`ZahirFT::NetCdf`] via [`file_type_from_metadata_name`].
    is_netcdf: bool,
    /// Passed to [`crate::render::kv_tables::format::format_value`] for `shape`-style arrays.
    pub max_array_inline: usize,
}

impl Default for WalkCtx {
    fn default() -> Self {
        Self {
            is_netcdf: false,
            max_array_inline: format::DEFAULT_MAX_ARRAY_INLINE,
        }
    }
}

impl WalkCtx {
    #[must_use]
    pub fn from_root_map(map_ref: &Map<String, Value>, max_array_inline: usize) -> Self {
        let is_netcdf = map_ref
            .get(WalkKeyVars::FILE_TYPE)
            .and_then(|v| v.as_str())
            .and_then(file_type_from_metadata_name)
            .is_some_and(|ft| ft == ZahirFT::NetCdf);
        Self {
            is_netcdf,
            max_array_inline: max_array_inline.max(1),
        }
    }
}

/// Drop [`WalkKeyVars::FILE_TYPE`] after [`WalkCtx::from_root_map`] so it is not rendered as metadata KV (category already shows type).
fn map_without_display_file_type(map_ref: &Map<String, Value>) -> Map<String, Value> {
    let mut m = map_ref.clone();
    m.remove(WalkKeyVars::FILE_TYPE);
    m
}

fn flatten_name_value_attribute_rows(
    arr_ref: &[Value],
    max_array_inline: usize,
) -> Option<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(arr_ref.len());
    for v in arr_ref {
        let obj = v.as_object()?;
        let name = obj.get(WalkKeyVars::NAME)?.as_str()?;
        let value = obj.get(WalkKeyVars::VALUE)?;
        // Keep identifier as-is (e.g. `_FillValue`); do not title-case via [`format::format_key`].
        out.push((
            name.to_string(),
            format::format_value(value, name, max_array_inline),
        ));
    }
    Some(out)
}

/// Like [`map_to_kv_rows`], but expands `attributes` name/value lists into flat rows for the same section.
fn map_to_kv_rows_flat_name_value_attributes(
    map_ref: &Map<String, Value>,
    exclude_key: Option<&str>,
    max_array_inline: usize,
) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (k, val) in map_ref {
        if exclude_key == Some(k.as_str()) {
            continue;
        }
        if k == WalkKeyVars::ATTRIBUTES
            && let Some(arr) = val.as_array()
            && let Some(flat) = flatten_name_value_attribute_rows(arr, max_array_inline)
        {
            rows.extend(flat);
            continue;
        }
        rows.push((
            format::format_key(k),
            format::format_value(val, k, max_array_inline),
        ));
    }
    rows
}

fn rows_for_table_record_map(
    map_ref: &Map<String, Value>,
    ctx: WalkCtx,
    exclude_key: Option<&str>,
) -> Vec<(String, String)> {
    if ctx.is_netcdf {
        map_to_kv_rows_flat_name_value_attributes(map_ref, exclude_key, ctx.max_array_inline)
    } else {
        map_to_kv_rows(map_ref, exclude_key, ctx.max_array_inline)
    }
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

fn push_contents_from_entries(
    sections_mut_ref: &mut Vec<Section>,
    arr_ref: &[Value],
    ctx: &WalkCtx,
) {
    if let Some((column_keys, columns, entries)) = object_array_to_contents_data(arr_ref) {
        sections_mut_ref.push(Section::Contents(ContentsSection {
            title: UI_STRINGS.tables.contents_title.to_string(),
            columns,
            column_keys,
            entries,
            sub_title: false,
        }));
    }
    spill_entry_row_detail_sections(sections_mut_ref, arr_ref, ctx);
}

/// Same as [`push_root_parts`] but returns a new vec. Used when parsing blobs in parallel.
#[must_use]
pub fn root_parts_sections(map_ref: &Map<String, Value>, max_array_inline: usize) -> Vec<Section> {
    let mut sections = Vec::new();
    push_root_parts(&mut sections, map_ref, max_array_inline);
    sections
}

/// Walk root map once; push sections in order (flat KV, schema, `sheet_stats`, `common_pivots`, `csv_metadata`, uniform object arrays, then each nested, then entries). Uses JSON key names (`SectionKeys`) in the match.
pub fn push_root_parts(
    sections_mut_ref: &mut Vec<Section>,
    map_ref: &Map<String, Value>,
    max_array_inline: usize,
) {
    let ctx = WalkCtx::from_root_map(map_ref, max_array_inline);
    let map_owned = map_without_display_file_type(map_ref);
    push_root_parts_inner(sections_mut_ref, &map_owned, ctx);
}

fn push_root_parts_inner(
    sections_mut_ref: &mut Vec<Section>,
    map_ref: &Map<String, Value>,
    ctx: WalkCtx,
) {
    let mut buckets = RootBuckets::default();
    for (k, v) in map_ref {
        buckets.classify(k, v, ctx.max_array_inline);
    }

    if !buckets.flat.is_empty() {
        sections_mut_ref.push(Section::KeyValue(KvSection {
            title: None,
            rows: buckets.flat,
            sub_title: false,
        }));
    }
    if let Some(v) = buckets.schema_val {
        schema::push_schema_section(sections_mut_ref, &v);
    }
    if let Some((key, obj)) = buckets.sheet_stats {
        sections_mut_ref.push(xlsx::sheet_stats_to_section(&key, &obj));
    }
    if let Some(values) = buckets.common_pivots.filter(|pivots| !pivots.is_empty()) {
        sections_mut_ref.push(Section::SingleColumnList(SingleColumnListSection {
            title: format::format_key(SectionKeys::COMMON_PIVOTS),
            values,
        }));
    }
    if let Some((key, meta)) = buckets.column_metadata_compact {
        column_metadata::push_column_metadata_sections(
            sections_mut_ref,
            &key,
            &meta,
            ctx.max_array_inline,
            None,
        );
    }
    if let Some(title) = buckets.column_metadata_legacy_title {
        column_metadata::push_legacy_column_metadata_notice(sections_mut_ref, Some(title), false);
    }
    for (array_key, arr) in buckets.record_object_arrays {
        push_tables_sections(sections_mut_ref, arr, ctx, array_key, None);
    }
    for (key, m) in buckets.nested {
        process_nested_map(sections_mut_ref, &key, &m, &ctx, None);
    }
    if let Some(arr) = buckets.entries {
        push_contents_from_entries(sections_mut_ref, &arr, &ctx);
    }
}

#[derive(Default)]
struct RootBuckets<'a> {
    flat: Vec<(String, String)>,
    nested: Vec<(String, Map<String, Value>)>,
    entries: Option<Vec<Value>>,
    schema_val: Option<Value>,
    sheet_stats: Option<(String, Map<String, Value>)>,
    common_pivots: Option<Vec<String>>,
    column_metadata_compact: Option<(String, Map<String, Value>)>,
    column_metadata_legacy_title: Option<String>,
    record_object_arrays: Vec<(&'a str, &'a Vec<Value>)>,
}

impl<'a> RootBuckets<'a> {
    fn push_flat(&mut self, key: &str, val: &Value, max_array_inline: usize) {
        self.flat.push((
            format::format_key(key),
            format::format_value(val, key, max_array_inline),
        ));
    }

    fn classify(&mut self, k: &'a str, v: &'a Value, max_array_inline: usize) {
        match k {
            SectionKeys::ENTRIES => self.entries = v.as_array().cloned(),
            SectionKeys::SCHEMA => self.schema_val = Some(v.clone()),
            SectionKeys::SHEET_STATS => self.classify_sheet_stats(k, v, max_array_inline),
            SectionKeys::COMMON_PIVOTS => self.classify_common_pivots(k, v, max_array_inline),
            SectionKeys::CSV_METADATA => self.classify_csv_metadata(k, v, max_array_inline),
            _ => self.classify_default(k, v, max_array_inline),
        }
    }

    fn classify_sheet_stats(&mut self, k: &str, v: &Value, max_array_inline: usize) {
        if let Some(obj) = v.as_object() {
            if xlsx::is_sheet_stats(obj) {
                self.sheet_stats = Some((k.to_string(), obj.clone()));
            } else {
                self.nested.push((k.to_string(), obj.clone()));
            }
            return;
        }
        self.push_flat(k, v, max_array_inline);
    }

    fn classify_common_pivots(&mut self, k: &str, v: &Value, max_array_inline: usize) {
        if let Some(arr) = v.as_array() {
            self.common_pivots = Some(
                arr.iter()
                    .map(|val| format::format_value(val, k, max_array_inline))
                    .collect(),
            );
            return;
        }
        self.push_flat(k, v, max_array_inline);
    }

    fn classify_csv_metadata(&mut self, k: &str, v: &Value, max_array_inline: usize) {
        if let Some(obj) = v.as_object() {
            if column_metadata::is_compact_column_metadata(obj) {
                self.column_metadata_compact = Some((k.to_string(), obj.clone()));
            } else if column_metadata::is_legacy_parallel_column_metadata(obj) {
                self.column_metadata_legacy_title = Some(format::format_key(k));
            } else {
                self.nested.push((k.to_string(), obj.clone()));
            }
            return;
        }
        self.push_flat(k, v, max_array_inline);
    }

    fn classify_default(&mut self, k: &'a str, v: &'a Value, max_array_inline: usize) {
        match v {
            Value::Array(arr) if array_is_record_table_list(k, arr) => {
                self.record_object_arrays.push((k, arr));
            }
            Value::Object(m) if !m.is_empty() => self.nested.push((k.to_string(), m.clone())),
            _ => self.push_flat(k, v, max_array_inline),
        }
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

/// Display title for a nested object: `parent · formatted key` when `parent` is set.
fn nested_object_section_title(parent_title: Option<&str>, section_key: &str) -> String {
    join_with_parent_prefix(parent_title, format::format_key(section_key))
}

fn join_with_parent_prefix(parent_title: Option<&str>, child_title: String) -> String {
    match parent_title {
        Some(p) if !p.is_empty() => format::join_dot([p, child_title.as_str()]),
        _ => child_title,
    }
}

fn expand_object_children_with_prefix(
    sections_mut_ref: &mut Vec<Section>,
    object_ref: &Map<String, Value>,
    ctx: &WalkCtx,
    parent_title: Option<&str>,
    exclude_key: Option<&str>,
) {
    for (k, val) in object_ref {
        if exclude_key == Some(k.as_str()) {
            continue;
        }
        match val {
            Value::Object(m) if !m.is_empty() => {
                process_nested_map(sections_mut_ref, k, m, ctx, parent_title);
            }
            Value::Array(arr) if !arr.is_empty() && array_is_record_table_list(k.as_str(), arr) => {
                push_tables_sections(sections_mut_ref, arr, *ctx, k.as_str(), parent_title);
            }
            _ => {}
        }
    }
}

/// For each `entries` row, expand in-cell objects and uniform object arrays into sections with titles prefixed by the row label (`name` / `path` / `Entries · i`).
fn spill_entry_row_detail_sections(
    sections_mut_ref: &mut Vec<Section>,
    arr_ref: &[Value],
    ctx: &WalkCtx,
) {
    for (i, v) in arr_ref.iter().enumerate() {
        let Some(obj) = v.as_object() else {
            continue;
        };
        let row_label = table_title_for_record(obj, Some(SectionKeys::ENTRIES), i);
        expand_object_children_with_prefix(
            sections_mut_ref,
            obj,
            ctx,
            Some(row_label.as_str()),
            None,
        );
    }
}

/// Key/paths first; otherwise a title from the JSON array key and row index; otherwise `Table` (no key).
fn table_title_for_record(
    obj_ref: &Map<String, Value>,
    array_key: Option<&str>,
    index: usize,
) -> String {
    if let Some(n) = obj_ref.get(WalkKeyVars::NAME).and_then(Value::as_str) {
        return n.to_string();
    }
    if let Some(p) = obj_ref.get(WalkKeyVars::PATH).and_then(Value::as_str) {
        return p.to_string();
    }
    if let Some(k) = array_key {
        return format::join_dot([format::format_key(k), index.to_string()]);
    }
    WalkKeyVars::DEFAULT_TABLE_TITLE.to_string()
}

fn push_tables_sections(
    sections_mut_ref: &mut Vec<Section>,
    arr_ref: &[Value],
    walk_ctx: WalkCtx,
    array_key: &str,
    title_row_prefix: Option<&str>,
) {
    for (i, v) in arr_ref.iter().enumerate() {
        let Some(v) = v.as_object() else {
            continue;
        };
        let table_name = join_with_parent_prefix(
            title_row_prefix,
            table_title_for_record(v, Some(array_key), i),
        );
        if column_metadata::is_compact_column_metadata(v) {
            let title_override = title_row_prefix
                .is_some_and(|p| !p.is_empty())
                .then_some(table_name.as_str());
            column_metadata::push_column_metadata_sections(
                sections_mut_ref,
                table_name.as_str(),
                v,
                walk_ctx.max_array_inline,
                title_override,
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
        let rows = rows_for_table_record_map(v, walk_ctx, Some(WalkKeyVars::COLUMNS));
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
                push_column_stats_sections(
                    sections_mut_ref,
                    &table_name,
                    &entries,
                    walk_ctx.max_array_inline,
                );
            }
        }
        // For table-record rows (datasets/variables/etc.), spill nested objects/record arrays
        // by shape instead of key name. Keep `columns` in its existing dedicated branch above.
        expand_object_children_with_prefix(
            sections_mut_ref,
            v,
            &walk_ctx,
            Some(table_name.as_str()),
            Some(WalkKeyVars::COLUMNS),
        );
    }
}

fn push_column_stats_sections(
    sections_mut_ref: &mut Vec<Section>,
    table_name_ref: &str,
    column_objs_ref: &[Value],
    max_array_inline: usize,
) {
    for col in column_objs_ref.iter().filter_map(Value::as_object) {
        let col_name = object_name_or(col, WalkKeyVars::DEFAULT_COLUMN_LABEL);
        for (stats_key, stats_val) in col {
            if stats_key == WalkKeyVars::NAME {
                continue;
            }
            if let Some(stats_obj) = stats_val.as_object() {
                let rows = map_to_kv_rows(stats_obj, None, max_array_inline);
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

/// Walk nested map once (entries, schema, `common_pivots`, nested child objects, uniform object arrays, rest flat KV). Push sections in order.
/// Child objects (e.g. a stats object under `tensor3d`) become their own key/value section instead of a single cell of stringified JSON.
/// `parent_title` is the formatted title of the containing map, used to build `parent · child` section titles.
pub fn process_nested_map(
    sections_mut_ref: &mut Vec<Section>,
    section_key_ref: &str,
    map_ref: &Map<String, Value>,
    ctx_ref: &WalkCtx,
    parent_title: Option<&str>,
) {
    let own_title = nested_object_section_title(parent_title, section_key_ref);
    if column_metadata::is_compact_column_metadata(map_ref) {
        column_metadata::push_column_metadata_sections(
            sections_mut_ref,
            section_key_ref,
            map_ref,
            ctx_ref.max_array_inline,
            Some(own_title.as_str()),
        );
        return;
    }
    if column_metadata::is_legacy_parallel_column_metadata(map_ref) {
        column_metadata::push_legacy_column_metadata_notice(
            sections_mut_ref,
            Some(own_title),
            false,
        );
        return;
    }

    let mut flat = Vec::new();
    let mut entries = None;
    let mut schema_val = None;
    let mut common_pivots: Option<Vec<String>> = None;
    let mut record_object_arrays: Vec<(&str, &Vec<Value>)> = Vec::new();
    let mut child_objects: Vec<(String, Map<String, Value>)> = Vec::new();

    for (k, v) in map_ref {
        match k.as_str() {
            SectionKeys::ENTRIES => entries = v.as_array().cloned(),
            SectionKeys::SCHEMA => schema_val = Some(v.clone()),
            SectionKeys::COMMON_PIVOTS => {
                if let Some(arr) = v.as_array() {
                    let mi = ctx_ref.max_array_inline;
                    common_pivots = Some(
                        arr.iter()
                            .map(|val| format::format_value(val, k, mi))
                            .collect(),
                    );
                } else {
                    flat.push((
                        format::format_key(k),
                        format::format_value(v, k, ctx_ref.max_array_inline),
                    ));
                }
            }
            _ => match v {
                Value::Object(m) if !m.is_empty() => {
                    child_objects.push((k.clone(), m.clone()));
                }
                Value::Array(arr) if array_is_record_table_list(k.as_str(), arr) => {
                    record_object_arrays.push((k.as_str(), arr));
                }
                _ => flat.push((
                    format::format_key(k),
                    format::format_value(v, k, ctx_ref.max_array_inline),
                )),
            },
        }
    }

    if !flat.is_empty() {
        sections_mut_ref.push(Section::KeyValue(KvSection {
            title: Some(own_title.clone()),
            rows: flat,
            sub_title: false,
        }));
    }
    for (child_key, child_map) in child_objects {
        process_nested_map(
            sections_mut_ref,
            &child_key,
            &child_map,
            ctx_ref,
            Some(own_title.as_str()),
        );
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
        push_contents_from_entries(sections_mut_ref, &arr, ctx_ref);
    }
    for (ak, arr) in record_object_arrays {
        push_tables_sections(sections_mut_ref, arr, *ctx_ref, ak, None);
    }
}
