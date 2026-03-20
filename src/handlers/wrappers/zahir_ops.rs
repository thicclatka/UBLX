//! `ZahirScan` integration: batch (sequential) and stream entry points.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use zahirscan::parsers::structured::{
    delimiter_byte_for_reader as zahir_delimiter_byte_for_reader,
    detect_delimiter_byte as zahir_detect_delimiter_byte,
};
use zahirscan::{
    FileType, Output, OutputSink, RuntimeConfig, ZahirScanResult, extract_zahir,
    extract_zahir_from_stream,
};

use super::nefax_ops;

use crate::config::UblxOpts;

pub type ZahirResult = ZahirScanResult;
pub type ZahirOutput = Output;
pub type ZahirOutputSink = OutputSink;

/// Parse a DB `category` string into [`FileType`] when it matches [`FileType::as_metadata_name`].
///
/// Delegates to [`FileType::from_metadata_name`] (zahirscan); full round-trip tests live there.
#[must_use]
pub fn file_type_from_metadata_name(s: &str) -> Option<FileType> {
    FileType::from_metadata_name(s)
}

/// Byte to pass to the Rust [`csv`](https://docs.rs/csv) crate’s [`csv::ReaderBuilder::delimiter`].
/// Same rules as zahirscan’s CSV metadata parser: `.tsv` / `.tab` → tab, `.psv` → pipe, else sniff.
#[must_use]
pub fn delimiter_byte_for_reader(content: &str, path_hint: &str) -> u8 {
    zahir_delimiter_byte_for_reader(content, path_hint)
}

/// Sniff delimiter from the first lines of `content` (comma, semicolon, tab, pipe, colon).
/// Use as a **fallback** when the file path has no recognized extension (see [`delimiter_from_path_for_viewer`]).
#[must_use]
pub fn detect_delimiter_byte(content: &str) -> u8 {
    zahir_detect_delimiter_byte(content)
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

fn extract_zahir_opts_from_ublx_opts(opts: &UblxOpts) -> RuntimeConfig {
    opts.zahir_runtime_config()
}

/// Run zahir on a full set of paths (sequential mode). Uses [`OutputMode::Full`] and the given config.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when the zahir scan fails.
pub fn run_zahir_batch(
    paths: &[impl AsRef<Path>],
    ublx_opts: &UblxOpts,
) -> Result<ZahirScanResult, anyhow::Error> {
    let config = extract_zahir_opts_from_ublx_opts(ublx_opts);
    let path_strings: Vec<String> = paths
        .iter()
        .map(|p| p.as_ref().to_string_lossy().into_owned())
        .collect();
    extract_zahir(
        path_strings,
        config.output_mode,
        Some(&config),
        None,
        &ZahirOutputSink::Collect,
    )
}

/// Run zahir on paths from a channel. Use `ZahirOutputSink::Collect` to get all outputs in the result (default).
/// Use `ZahirOutputSink::Channel(tx)` to stream each `(path, Output)` to a receiver so ublx can write to the DB incrementally.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when the zahir scan fails.
pub fn run_zahir_from_stream(
    paths_rx: Receiver<String>,
    ublx_opts: &UblxOpts,
    output_sink: ZahirOutputSink,
) -> Result<ZahirScanResult, anyhow::Error> {
    let config = extract_zahir_opts_from_ublx_opts(ublx_opts);
    extract_zahir_from_stream(
        &paths_rx,
        config.output_mode,
        Some(&config),
        None,
        &output_sink,
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

/// Convert a zahir output to a JSON string.
#[must_use]
pub fn zahir_output_to_json(output: Option<&ZahirOutput>) -> String {
    output
        .and_then(|o| serde_json::to_string(o).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod file_type_from_metadata_name_tests {
    use super::{FileType, file_type_from_metadata_name};

    #[test]
    fn wrapper_matches_zahirscan_api() {
        assert_eq!(file_type_from_metadata_name("CSV"), Some(FileType::Csv));
        assert_eq!(
            file_type_from_metadata_name("Markdown"),
            Some(FileType::Markdown)
        );
    }

    #[test]
    fn non_zahir_categories_miss() {
        assert_eq!(file_type_from_metadata_name("Directory"), None);
        assert_eq!(file_type_from_metadata_name("not a label"), None);
    }
}
