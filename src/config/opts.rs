//! Options for ublx, extending per-tool opts (e.g. NefaxOpts, zahirscan RuntimeConfig).
//!
//! Worker pool: [Self::max_workers_available] is derived from nefax tuning (drive-type aware).
//! When >= [Self::SEQUENTIAL_THRESHOLD], workers are split by ratio (or overrides) across nefax, zahir, and ublx.
//! When below threshold, sequential mode: run phases one after another, each using all available workers.
//! For zahir-only (e.g. single file, no nefax), use [Self::for_zahir_only] with a chosen max (e.g. from tuning).
//!
//! A config file in `dir` (`.ublx.toml` or `ublx.toml`, whichever exists) is loaded when present; only keys that exist in the file overlay the built opts.

use std::fs;
use std::path::Path;

use log::warn;
use nefaxer::{NefaxOpts, tuning_for_path};
use serde::Deserialize;
use zahirscan::{OutputMode, RuntimeConfig};

use crate::handlers::nefax_ops;

use super::paths::UblxPaths;

/// At or above this many workers we set [UblxOpts::streaming] to true (callback path for nefax).
pub const STREAMING_THRESHOLD: usize = 6;

/// Keys that can appear in `.ublx.toml`; only present keys override [UblxOpts].
/// Streaming and worker counts are optimized (threshold-derived), not configurable here.
/// Exclude is appended to nefax [NefaxOpts::exclude]. Ublx does not add ".*"; use exclude in toml if you want that. Nefaxer and zahir each have their own hidden-file behavior.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct UblxOptsOverlay {
    /// Extra paths/patterns to exclude from indexing (appended to nefax [NefaxOpts::exclude]). When present, applied.
    exclude: Option<Vec<String>>,
    /// When true, show hidden files: do not add ".*" to nefax, and zahir includes hidden. When false, exclude ".*" and zahir skips hidden.
    #[serde(rename = "show_hidden_files")]
    show_hidden_files: Option<bool>,
    /// When true, nefaxer computes blake3 hash for files (slower, more accurate change detection). Sets [NefaxOpts::with_hash].
    hash: Option<bool>,
}

const HIDDEN_EXCLUDE_PATTERN: &str = ".*";

/// Options for ublx. Extends [NefaxOpts] and [RuntimeConfig]; owns worker-pool sizing and streaming.
#[derive(Clone, Debug)]
pub struct UblxOpts {
    /// Options passed to the nefaxer indexer (base; use [Self::nefax_opts_with_workers] for run).
    pub nefax: NefaxOpts,
    /// ZahirScan runtime config. Use [Self::zahir_runtime_config] to get config with ublx overrides applied.
    #[allow(dead_code)]
    pub zahir: RuntimeConfig,
    /// Max workers suggested by tuning (drive-type aware). From nefax [tuning_for_path] in [Self::for_dir].
    pub max_workers_available: usize,
    /// Override workers for nefaxer. When unset and >= [SEQUENTIAL_THRESHOLD], uses share from 1:1:1 ratio.
    pub nefax_workers_override: Option<usize>,
    /// Override workers for zahirscan. When unset and >= threshold, uses share from ratio.
    #[allow(dead_code)]
    pub zahir_workers_override: Option<usize>,
    /// Override workers for ublx (main process / other work). When unset and >= threshold, remainder from ratio.
    #[allow(dead_code)]
    pub ublx_workers_override: Option<usize>,
    /// Use streaming (callback) path for nefax when true.
    pub streaming: bool,
}

impl UblxOpts {
    /// Build ublx options for indexing `dir`. [Self::max_workers_available] comes from [tuning_for_path](nefaxer::tuning_for_path).
    /// Zahir config is loaded with [RuntimeConfig::new]. [Self::streaming] is set true when workers >= [STREAMING_THRESHOLD].
    /// Load overlay from config path (`.ublx.toml` or `ublx.toml`, whichever exists per [UblxPaths::toml_path]). Returns None if missing or parse error.
    fn load_ublx_toml(path: Option<std::path::PathBuf>) -> Option<UblxOptsOverlay> {
        let path = path?;
        let s = fs::read_to_string(&path).ok()?;
        match toml::from_str::<UblxOptsOverlay>(&s) {
            Ok(overlay) => Some(overlay),
            Err(e) => {
                warn!("{}: parse error, ignoring: {}", path.display(), e);
                None
            }
        }
    }

    fn apply_overlay(&mut self, overlay: UblxOptsOverlay) {
        if let Some(extra) = overlay.exclude {
            self.nefax.exclude.extend(extra);
        }
        if let Some(show_hidden) = overlay.show_hidden_files {
            if show_hidden {
                self.zahir.ignore_hidden_files = false;
            } else {
                self.nefax.exclude.push(HIDDEN_EXCLUDE_PATTERN.to_string());
                self.zahir.ignore_hidden_files = true;
            }
        }
        if let Some(hash) = overlay.hash {
            self.nefax.with_hash = hash;
        }
    }

    /// Build ublx options for indexing `dir`. [Self::max_workers_available] comes from [tuning_for_path](nefaxer::tuning_for_path).
    /// Zahir config is loaded with [RuntimeConfig::new]. [Self::streaming] is set true when workers >= [STREAMING_THRESHOLD].
    /// If a config file exists (`paths.toml_path()`: `.ublx.toml` or `ublx.toml`), only keys present in it overlay these opts.
    pub fn for_dir(
        dir: &Path,
        paths: &UblxPaths,
        nefax_workers_override: Option<usize>,
        zahir_workers_override: Option<usize>,
        ublx_workers_override: Option<usize>,
    ) -> Self {
        let exclude = paths.exclude();
        let (num_threads, _drive_type, _use_parallel_walk) = tuning_for_path(dir, None);
        let nefax = nefax_ops::pre_opts_for_nefaxer(dir, &exclude);
        let zahir = RuntimeConfig::new();
        let streaming = num_threads >= STREAMING_THRESHOLD;
        let mut opts = Self {
            nefax,
            zahir,
            max_workers_available: num_threads,
            nefax_workers_override,
            zahir_workers_override,
            ublx_workers_override,
            streaming,
        };
        if let Some(overlay) = Self::load_ublx_toml(paths.toml_path()) {
            opts.apply_overlay(overlay);
        }
        opts
    }

    /// Build opts when running zahir only (e.g. single file, no nefax). You supply [Self::max_workers_available] (e.g. from tuning on a path); all are used for zahir.
    /// [Self::streaming] is set true when workers >= [STREAMING_THRESHOLD].
    #[allow(dead_code)]
    pub fn for_zahir_only(max_workers_available: usize, zahir: RuntimeConfig) -> Self {
        let nefax = NefaxOpts::default();
        let streaming = max_workers_available >= STREAMING_THRESHOLD;
        Self {
            nefax,
            zahir,
            max_workers_available,
            nefax_workers_override: Some(0),
            zahir_workers_override: Some(max_workers_available),
            ublx_workers_override: Some(0),
            streaming,
        }
    }

    /// True when [Self::max_workers_available] < [SEQUENTIAL_THRESHOLD]: run phases sequentially, each phase using all workers.
    pub fn is_sequential_mode(&self) -> bool {
        self.max_workers_available < STREAMING_THRESHOLD
    }

    fn default_share_1_1_1(&self) -> (usize, usize, usize) {
        let n = self.max_workers_available;
        let third = n / 3;
        let nefax = third;
        let zahir = third;
        let ublx = n.saturating_sub(nefax).saturating_sub(zahir);
        (nefax, zahir, ublx)
    }

    fn effective_workers_for(&self, override_val: Option<usize>, default_share: usize) -> usize {
        if self.is_sequential_mode() {
            return self.max_workers_available;
        }
        override_val
            .unwrap_or(default_share)
            .min(self.max_workers_available)
    }

    /// Workers to use for nefaxer. Sequential mode: all [Self::max_workers_available]; else override or ratio share.
    pub fn effective_nefax_workers(&self) -> usize {
        let (n, _, _) = self.default_share_1_1_1();
        self.effective_workers_for(self.nefax_workers_override, n)
    }

    /// Workers to use for zahirscan. Sequential mode: all available; else override or ratio share.
    #[allow(dead_code)]
    pub fn effective_zahir_workers(&self) -> usize {
        let (_, z, _) = self.default_share_1_1_1();
        self.effective_workers_for(self.zahir_workers_override, z)
    }

    /// Workers reserved for ublx (main process / other work). Sequential mode: all available; else override or remainder.
    #[allow(dead_code)]
    pub fn effective_ublx_workers(&self) -> usize {
        let (_, _, u) = self.default_share_1_1_1();
        self.effective_workers_for(self.ublx_workers_override, u)
    }

    /// Reference to the inner [NefaxOpts] (base, no worker override applied).
    #[allow(dead_code)]
    pub fn nefax_opts(&self) -> &NefaxOpts {
        &self.nefax
    }

    /// [NefaxOpts] with [Self::effective_nefax_workers] applied to `num_threads` for use with [nefax_ops::run_nefaxer].
    #[allow(dead_code)]
    pub fn nefax_opts_with_workers(&self) -> NefaxOpts {
        let mut opts = self.nefax.clone();
        opts.num_threads = Some(self.effective_nefax_workers());
        opts
    }

    /// ZahirScan runtime config with ublx overrides: [OutputMode::Full], [Self::effective_zahir_workers] for `max_workers`.
    #[allow(dead_code)]
    pub fn zahir_runtime_config(&self) -> RuntimeConfig {
        let mut config = self.zahir.clone();
        config.output_mode = OutputMode::Full;
        config.max_workers = self.effective_zahir_workers();
        config
    }
}
