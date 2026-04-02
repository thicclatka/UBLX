//! `ZahirScan` integration: batch (sequential) and stream entry points.
//!
//! Sections: **path / file-type hints** (extension + linguist without full extract), **`extract_zahir`** entry
//! points, **result indexing**, **JSON for DB**.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use log::debug;
use serde_json::Value;
use zahirscan;

use crate::config::UblxOpts;

use super::nefax_ops;

pub type ZahirResult = zahirscan::ZahirScanResult;
pub type ZahirOutput = zahirscan::Output;
pub type ZahirOutputSink = zahirscan::OutputSink;
pub type ZahirFT = zahirscan::FileType;
pub type ZahirOutputMode = zahirscan::OutputMode;
pub type ZahirRC = zahirscan::RuntimeConfig;

/// Safe ffprobe invocation (JSON format/streams). Delegates to [`zahirscan::utils::ffprobe_handler::run_ffprobe_safe`].
pub use zahirscan::utils::ffprobe_handler::run_ffprobe_safe;

/// Parse a DB `category` string into [`FileType`] when it matches [`FileType::as_metadata_name`].
///
/// Delegates to [`FileType::from_metadata_name`] (zahirscan); full round-trip tests live there.
#[must_use]
pub fn file_type_from_metadata_name(s: &str) -> Option<zahirscan::FileType> {
    zahirscan::FileType::from_metadata_name(s)
}

/// Sniff delimiter from the first lines of `content` (comma, semicolon, tab, pipe, colon).
/// Use as a **fallback** when the file path has no recognized extension (see [`delimiter_from_path_for_viewer`]).
#[must_use]
pub fn detect_delimiter_byte(content: &str) -> u8 {
    zahirscan::parsers::structured::detect_delimiter_byte(content)
}

/// Delimiter implied by the path’s extension, when it matches zahirscan’s delimited types.
/// `.csv` → comma, `.tsv` / `.tab` → tab, `.psv` → pipe; otherwise [`None`] (caller should use [`detect_delimiter_byte`]).
#[must_use]
pub fn delimiter_from_path_for_viewer(path: &str) -> Option<u8> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)?;
    match ext.as_str() {
        "csv" => Some(b','),
        "tsv" | "tab" => Some(b'\t'),
        "psv" => Some(b'|'),
        _ => None,
    }
}

// --- Path / file-type hints (no full extract) --------------------------------

#[must_use]
fn metadata_name_from_detect_key(key: &str) -> Option<String> {
    let ft = zahirscan::utils::filetypes::detect_file_type(key);
    (ft != zahirscan::FileType::Unknown).then(|| ft.as_metadata_name().to_string())
}

/// Metadata name string for [`FileType`] from path/extension only (ZahirScan’s [`detect_file_type`]), without a full extract.
///
/// **Caveat:** zahirscan’s linguist fallback uses `Path::new(path_str).exists()`, which is relative to **process cwd**.
/// For indexed trees when cwd ≠ project root (e.g. `ublx /path/to/repo`), use [`zahir_metadata_name_from_indexed_file`].
#[must_use]
pub fn zahir_metadata_name_from_path_hint(path_str: &str) -> Option<String> {
    metadata_name_from_detect_key(path_str)
}

/// Like [`zahir_metadata_name_from_path_hint`], but uses `full_path` (e.g. `dir_to_ublx.join(rel)`) for
/// [`detect_file_type`] when that path exists so `.py` / `.rs` / linguist work regardless of cwd.
#[must_use]
pub fn zahir_metadata_name_from_indexed_file(full_path: &Path, path_str: &str) -> Option<String> {
    let key = if full_path.exists() {
        full_path.to_string_lossy().into_owned()
    } else {
        path_str.to_string()
    };
    metadata_name_from_detect_key(&key)
}

// --- extract_zahir entry points ---------------------------------------------

/// True if we should run zahir on this path (new or mtime changed). Skip when prior exists and mtime is unchanged.
#[must_use]
pub fn needs_zahir(
    prior_nefax: Option<&nefax_ops::NefaxResult>,
    path: &PathBuf,
    current_mtime_ns: i64,
) -> bool {
    match prior_nefax.and_then(|p| p.get(path)) {
        Some(prior_meta) => prior_meta.mtime_ns != current_mtime_ns,
        None => true,
    }
}

/// When `paths` is empty, log and return a default result so callers skip `extract_zahir`.
fn zahir_empty_when_no_paths(
    paths: &[String],
    mode_label: &'static str,
) -> Option<zahirscan::ZahirScanResult> {
    if paths.is_empty() {
        debug!("zahir {mode_label}: no paths received, returning empty result");
        Some(zahirscan::ZahirScanResult::default())
    } else {
        None
    }
}

/// Run zahir on a full set of paths (sequential mode). Uses [`OutputMode::Full`] and the given config.
///
/// If the path list is empty, returns [`ZahirScanResult::default`] without calling zahirscan.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when zahirscan fails (including when no paths are scannable).
pub fn run_zahir_batch(
    paths: &[impl AsRef<Path>],
    ublx_opts: &UblxOpts,
) -> Result<zahirscan::ZahirScanResult, anyhow::Error> {
    let config = ublx_opts.zahir_runtime_config();
    let path_strings: Vec<String> = paths
        .iter()
        .map(|p| p.as_ref().to_string_lossy().into_owned())
        .collect();
    if let Some(empty) = zahir_empty_when_no_paths(&path_strings, "batch") {
        return Ok(empty);
    }
    zahirscan::extract_zahir(
        path_strings,
        config.output_mode,
        Some(&config),
        None,
        &ZahirOutputSink::Collect,
    )
}

/// Run zahir on paths from a channel. Drains `paths_rx` until closed (same as [`zahirscan::extract_zahir_from_stream`]), then runs [`extract_zahir`].
/// Use `ZahirOutputSink::Collect` to get all outputs in the result (default).
/// Use `ZahirOutputSink::Channel(tx)` to stream each `(path, Output)` to a receiver so ublx can write to the DB incrementally.
///
/// If no paths were received, returns [`ZahirScanResult::default`] without calling zahirscan.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when zahirscan fails (including when no paths are scannable).
pub fn run_zahir_from_stream(
    paths_rx: &Receiver<String>,
    ublx_opts: &UblxOpts,
    output_sink: &ZahirOutputSink,
) -> Result<zahirscan::ZahirScanResult, anyhow::Error> {
    let config = ublx_opts.zahir_runtime_config();
    let path_strings: Vec<String> = paths_rx.iter().collect();
    if let Some(empty) = zahir_empty_when_no_paths(&path_strings, "stream") {
        return Ok(empty);
    }
    zahirscan::extract_zahir(
        path_strings,
        config.output_mode,
        Some(&config),
        None,
        output_sink,
    )
}

/// Zahir output by path from a zahir result. Keys are path strings.
/// If `root` is `Some`, keys are relative to `root` (so they line up with nefaxer); otherwise keys are absolute (source as-is).
#[must_use]
pub fn get_zahir_output_by_path<'a>(
    zahir_result: &'a ZahirResult,
    dir_to_ublx_abs: Option<&Path>,
) -> HashMap<String, &'a ZahirOutput> {
    zahir_result
        .outputs
        .iter()
        .filter_map(|o| {
            let s = o.source.as_ref()?;
            let key = match dir_to_ublx_abs {
                Some(r) => Path::new(s)
                    .strip_prefix(r)
                    .ok()?
                    .to_string_lossy()
                    .into_owned(),
                None => s.clone(),
            };
            Some((key, o))
        })
        .collect()
}

// --- JSON for DB -------------------------------------------------------------

/// Convert a zahir output to a JSON string (no path-based `file_type` fill-in).
#[must_use]
pub fn zahir_output_to_json(output: Option<&ZahirOutput>) -> String {
    output
        .and_then(|o| serde_json::to_string(o).ok())
        .unwrap_or_default()
}

fn zahir_json_needs_path_file_type(v: &Value) -> bool {
    match v.get("file_type") {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    }
}

/// Merge path-based `file_type` when it is missing or empty (uses indexed path for cwd-safe detection).
fn inject_path_detected_file_type(v: &mut Value, full_path: &Path, path_str: &str) {
    if !zahir_json_needs_path_file_type(v) {
        return;
    }
    let Some(name) = zahir_metadata_name_from_indexed_file(full_path, path_str) else {
        return;
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("file_type".to_string(), Value::String(name));
    }
}

/// Serialize [`ZahirOutput`] for DB storage. When `file_type` is absent or empty, sets it from
/// zahirscan’s [`detect_file_type`] using `full_path` when it exists (same labels as full extract).
#[must_use]
pub fn zahir_output_to_json_for_path(
    output: Option<&ZahirOutput>,
    full_path: &Path,
    path_str: &str,
) -> String {
    let Some(o) = output else {
        return String::new();
    };
    let Ok(mut v) = serde_json::to_value(o) else {
        return zahir_output_to_json(Some(o));
    };
    inject_path_detected_file_type(&mut v, full_path, path_str);
    serde_json::to_string(&v).unwrap_or_default()
}
