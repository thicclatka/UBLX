//! ZahirScan integration: batch (sequential) and stream entry points.

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;

use zahirscan::{
    FileType, Output, RuntimeConfig, ZahirScanResult, extract_zahir, extract_zahir_from_stream,
};

use crate::config::UblxOpts;

pub type ZahirResult = ZahirScanResult;
pub type ZahirOutput = Output;
pub type ZahirFileType = FileType;

fn extract_zahir_opts_from_ublx_opts(opts: &UblxOpts) -> RuntimeConfig {
    opts.zahir_runtime_config()
}

/// Run zahir on a full set of paths (sequential mode). Uses [OutputMode::Full] and the given config.
pub fn run_zahir_batch(
    paths: &[impl AsRef<Path>],
    ublx_opts: &UblxOpts,
) -> Result<ZahirScanResult, anyhow::Error> {
    let config = extract_zahir_opts_from_ublx_opts(ublx_opts);
    let path_strings: Vec<String> = paths
        .iter()
        .map(|p| p.as_ref().to_string_lossy().into_owned())
        .collect();
    extract_zahir(path_strings, config.output_mode, Some(&config), None, None)
}

/// Run zahir on paths received from a channel (streaming mode). Drains `paths_rx` until the sender is dropped.
/// Uses [OutputMode::Full] and the given config.
pub fn run_zahir_from_stream(
    paths_rx: Receiver<String>,
    ublx_opts: &UblxOpts,
) -> Result<ZahirScanResult, anyhow::Error> {
    let config = extract_zahir_opts_from_ublx_opts(ublx_opts);
    extract_zahir_from_stream(paths_rx, config.output_mode, Some(&config), None, None)
}

/// Zahir output by path from a zahir result. Keys are path strings.
/// If `root` is `Some`, keys are relative to `root` (so they line up with nefaxer); otherwise keys are absolute (source as-is).
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
pub fn zahir_output_to_json(output: Option<&ZahirOutput>) -> String {
    output
        .and_then(|o| serde_json::to_string(o).ok())
        .unwrap_or_default()
}
