//! ZahirScan integration: batch (sequential) and stream entry points.

use std::path::Path;
use std::sync::mpsc::Receiver;

use zahirscan::{
    Output, OutputMode, RuntimeConfig, ZahirScanResult, extract_zahir, extract_zahir_from_stream,
};

use crate::config::UblxOpts;

pub type ZahirResult = ZahirScanResult;
pub type ZahirOutput = Output;

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
    extract_zahir(path_strings, OutputMode::Full, Some(&config), None, None)
}

/// Run zahir on paths received from a channel (streaming mode). Drains `paths_rx` until the sender is dropped.
/// Uses [OutputMode::Full] and the given config.
pub fn run_zahir_from_stream(
    paths_rx: Receiver<String>,
    ublx_opts: &UblxOpts,
) -> Result<ZahirScanResult, anyhow::Error> {
    let config = extract_zahir_opts_from_ublx_opts(ublx_opts);
    extract_zahir_from_stream(paths_rx, OutputMode::Full, Some(&config), None, None)
}
