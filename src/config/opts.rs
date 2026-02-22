//! Options for ublx, extending per-tool opts (e.g. NefaxOpts, zahirscan RuntimeConfig).
//!
//! Worker pool: [Self::max_workers_available] is derived from nefax tuning (drive-type aware).
//! When >= [Self::SEQUENTIAL_THRESHOLD], workers are split by ratio (or overrides) across nefax, zahir, and ublx.
//! When below threshold, sequential mode: run phases one after another, each using all available workers.
//! For zahir-only (e.g. single file, no nefax), use [Self::for_zahir_only] with a chosen max (e.g. from tuning).
//!
//! Config overlay: global `~/.config/ublx/ublx.toml` (if present) is applied first, then local (`.ublx.toml` or `ublx.toml` in the indexed dir). Only keys present in each file override defaults.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use log::warn;
use nefaxer::NefaxOpts;
use serde::{Deserialize, Serialize};
use zahirscan::{OutputMode, RuntimeConfig};

use super::paths::UblxPaths;
use crate::handlers::nefax_ops::{NefaxDriveType, pre_opts_for_nefaxer};

/// Cached disk/tuning settings stored in the ublx DB so we can skip disk check when .ublx exists.
#[derive(Clone, Debug)]
pub struct UblxSettings {
    pub num_threads: usize,
    pub drive_type: String,
    pub parallel_walk: bool,
    /// When global config exists: "local" = use local (dir) config; "global" = use global. Stored in .ublx.
    pub config_source: Option<String>,
}

/// Parse drive type string from DB/cache ("SSD", "HDD", "Network", "Unknown").
pub fn parse_drive_type(s: &str) -> NefaxDriveType {
    match s {
        "SSD" => NefaxDriveType::SSD,
        "HDD" => NefaxDriveType::HDD,
        "Network" => NefaxDriveType::Network,
        _ => NefaxDriveType::Unknown,
    }
}

fn drive_type_to_string(d: NefaxDriveType) -> &'static str {
    match d {
        NefaxDriveType::SSD => "SSD",
        NefaxDriveType::HDD => "HDD",
        NefaxDriveType::Network => "Network",
        NefaxDriveType::Unknown => "Unknown",
    }
}

/// At or above this many workers we set [UblxOpts::streaming] to true (callback path for nefax).
pub const STREAMING_THRESHOLD: usize = 6;

/// Layout pane percentages (0–100). Used for main 3-pane split: left (categories), middle (contents), right (preview). Not applied on the fly yet; config is read at startup.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct LayoutOverlay {
    pub left_pct: u16,
    pub middle_pct: u16,
    pub right_pct: u16,
}

impl Default for LayoutOverlay {
    fn default() -> Self {
        Self {
            left_pct: 20,
            middle_pct: 30,
            right_pct: 50,
        }
    }
}

/// Hot-reloadable options read from config files. Only present keys override; used for global + local overlay.
/// Apply in order: defaults → global `~/.config/ublx/ublx.toml` → local `.ublx.toml` or `ublx.toml` in indexed dir.
/// Reserved for future: theme and other visual elements (not used yet).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct UblxOverlay {
    /// Extra paths/patterns to exclude from indexing (appended to nefax [NefaxOpts::exclude]).
    pub exclude: Option<Vec<String>>,
    /// When true, show hidden files; when false, exclude ".*" and zahir skips hidden.
    #[serde(rename = "show_hidden_files")]
    pub show_hidden_files: Option<bool>,
    /// When true, nefaxer computes blake3 hash for files (slower, more accurate change detection).
    pub hash: Option<bool>,
    /// Reserved for theme selection (e.g. "dark", "light"). Not applied yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// When true, do not paint app background; terminal default (or transparency) shows through.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent: Option<bool>,
    /// Optional [layout] section: left/middle/right pane percentages (e.g. left_pct = 20, middle_pct = 30, right_pct = 50). Not applied on the fly yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutOverlay>,
}

impl UblxOverlay {
    /// Overlay with values from `other`; only fields set in `other` are applied (local overrides global when merging).
    pub fn merge_from(&mut self, other: &UblxOverlay) {
        if other.exclude.is_some() {
            self.exclude = other.exclude.clone();
        }
        if other.show_hidden_files.is_some() {
            self.show_hidden_files = other.show_hidden_files;
        }
        if other.hash.is_some() {
            self.hash = other.hash;
        }
        if other.theme.is_some() {
            self.theme = other.theme.clone();
        }
        if other.transparent.is_some() {
            self.transparent = other.transparent;
        }
        if other.layout.is_some() {
            self.layout = other.layout.clone();
        }
    }

    /// Merge global then local into one overlay (local wins). Used to apply once and to cache the effective config.
    pub fn merge(global: Option<UblxOverlay>, local: Option<UblxOverlay>) -> UblxOverlay {
        let mut out = UblxOverlay::default();
        if let Some(g) = global {
            out.merge_from(&g);
        }
        if let Some(l) = local {
            out.merge_from(&l);
        }
        out
    }
}

const HIDDEN_EXCLUDE_PATTERN: &str = ".*";

/// Options for ublx. Extends [NefaxOpts] and [RuntimeConfig]; owns worker-pool sizing and streaming.
#[derive(Clone, Debug)]
pub struct UblxOpts {
    /// Options passed to the nefaxer indexer (base; use [Self::nefax_opts_with_workers] for run).
    pub nefax: NefaxOpts,
    /// ZahirScan runtime config. Use [Self::zahir_runtime_config] to get config with ublx overrides applied.
    pub zahir: RuntimeConfig,
    /// Max workers suggested by tuning (drive-type aware). From nefax [tuning_for_path] in [Self::for_dir].
    pub max_workers_available: usize,
    /// Override workers for nefaxer. When unset and >= [SEQUENTIAL_THRESHOLD], uses share from 1:1:1 ratio.
    pub nefax_workers_override: Option<usize>,
    /// Override workers for zahirscan. When unset and >= threshold, uses share from ratio.
    pub zahir_workers_override: Option<usize>,
    /// Override workers for ublx (main process / other work). When unset and >= threshold, remainder from ratio.
    #[allow(dead_code)]
    pub ublx_workers_override: Option<usize>,
    /// Use streaming (callback) path for nefax when true.
    pub streaming: bool,
    /// When global config exists: "local" | "global". Preserved from cached_settings for writing back to DB.
    pub config_source: Option<String>,
    /// Theme name (e.g. "default"). From config overlay; used by layout::themes::get.
    pub theme: Option<String>,
    /// When true, skip painting app background so terminal default/transparency shows.
    pub transparent: bool,
    /// Left/middle/right pane percentages (0–100). From config [layout]; not applied on the fly yet. Default 20/30/50.
    pub layout: LayoutOverlay,
}

impl UblxOpts {
    /// Build ublx options for indexing `dir`. [Self::max_workers_available] comes from [tuning_for_path](nefaxer::tuning_for_path).
    /// Zahir config is loaded with [RuntimeConfig::new]. [Self::streaming] is set true when workers >= [STREAMING_THRESHOLD].
    /// Load overlay from a single toml file. Returns None if path is None, file missing, or parse error.
    fn load_ublx_toml(path: Option<std::path::PathBuf>) -> Option<UblxOverlay> {
        let path = path?;
        let s = fs::read_to_string(&path).ok()?;
        match toml::from_str::<UblxOverlay>(&s) {
            Ok(overlay) => Some(overlay),
            Err(e) => {
                warn!("{}: parse error, ignoring: {}", path.display(), e);
                None
            }
        }
    }

    /// Load the last applied overlay from cache (`cache_dir()/last_config.toml`). Use as fallback when hot reload gets invalid config.
    #[allow(dead_code)]
    pub fn load_overlay_from_cache(ublx_paths: &UblxPaths) -> Option<UblxOverlay> {
        Self::load_ublx_toml(ublx_paths.last_applied_config_path())
    }

    /// Load overlay from a single toml file path. Returns default if path is None, file missing, or parse error.
    pub fn load_overlay_from_path(path: Option<PathBuf>) -> UblxOverlay {
        Self::load_ublx_toml(path).unwrap_or_default()
    }

    /// Save overlay to cache dir as `last_config.toml`. Creates cache dir if needed. Logs and ignores write errors.
    fn save_overlay_to_cache(ublx_paths: &UblxPaths, overlay: &UblxOverlay) {
        let Some(path) = ublx_paths.last_applied_config_path() else {
            return;
        };
        if let Some(parent) = path.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            warn!("could not create cache dir {}: {}", parent.display(), e);
            return;
        }
        match toml::to_string_pretty(overlay) {
            Ok(s) => {
                if let Err(e) = fs::write(&path, s) {
                    warn!("could not write cache config {}: {}", path.display(), e);
                }
            }
            Err(e) => warn!("could not serialize overlay for cache: {}", e),
        }
    }

    fn apply_overlay(&mut self, overlay: UblxOverlay) {
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
        if overlay.theme.is_some() {
            self.theme = overlay.theme;
        }
        if overlay.transparent.is_some() {
            self.transparent = overlay.transparent.unwrap_or(false);
        }
        if overlay.layout.is_some() {
            self.layout = overlay.layout.clone().unwrap_or_default();
        }
    }

    /// Build ublx options for indexing `dir`. When `cached_settings` is `Some`, use those values and skip disk check; otherwise call [tuning_for_path](nefaxer::tuning_for_path).
    /// Zahir config is loaded with [RuntimeConfig::new]. [Self::streaming] is set true when workers >= [STREAMING_THRESHOLD].
    /// If a config file exists (`paths.toml_path()`: `.ublx.toml` or `ublx.toml`), only keys present in it overlay these opts.
    pub fn for_dir(
        dir_to_ublx: &Path,
        ublx_paths: &UblxPaths,
        nefax_workers_override: Option<usize>,
        zahir_workers_override: Option<usize>,
        ublx_workers_override: Option<usize>,
        cached_settings: Option<&UblxSettings>,
    ) -> Self {
        let exclude = ublx_paths.exclude();
        let nefax = pre_opts_for_nefaxer(dir_to_ublx, &exclude, cached_settings);
        let num_threads = nefax.num_threads.unwrap_or(1);
        let zahir = RuntimeConfig::new();
        let streaming = num_threads >= STREAMING_THRESHOLD;
        let config_source = cached_settings.and_then(|s| s.config_source.clone());
        let mut opts = Self {
            nefax,
            zahir,
            max_workers_available: num_threads,
            nefax_workers_override,
            zahir_workers_override,
            ublx_workers_override,
            streaming,
            config_source,
            theme: None,
            transparent: false,
            layout: LayoutOverlay::default(),
        };
        // Global then local: both accessible; local overrides global when both exist.
        let global = Self::load_ublx_toml(ublx_paths.global_config());
        let local = Self::load_ublx_toml(ublx_paths.toml_path());
        let merged = UblxOverlay::merge(global, local);
        opts.apply_overlay(merged.clone());
        Self::save_overlay_to_cache(ublx_paths, &merged);
        opts
    }

    /// Build [UblxSettings] from this opts for writing to the ublx DB (so next run can skip disk check).
    pub fn to_ublx_settings(&self) -> UblxSettings {
        UblxSettings {
            num_threads: self.max_workers_available,
            drive_type: self
                .nefax
                .drive_type
                .map(drive_type_to_string)
                .unwrap_or("Unknown")
                .to_string(),
            parallel_walk: self.nefax.use_parallel_walk.unwrap_or(false),
            config_source: self.config_source.clone(),
        }
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
            config_source: None,
            theme: None,
            transparent: false,
            layout: LayoutOverlay::default(),
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

    /// Streaming: 2 workers nefaxer, 2 ublx (e.g. future live TUI), rest for zahirscan.
    fn default_share_streaming(&self) -> (usize, usize, usize) {
        let n = self.max_workers_available;
        let nefax = 2.min(n);
        let ublx = 2.min(n.saturating_sub(nefax));
        let zahir = n.saturating_sub(nefax).saturating_sub(ublx);
        (nefax, zahir, ublx)
    }

    fn default_share(&self) -> (usize, usize, usize) {
        if self.streaming {
            self.default_share_streaming()
        } else {
            self.default_share_1_1_1()
        }
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
        let (n, _, _) = self.default_share();
        self.effective_workers_for(self.nefax_workers_override, n)
    }

    /// Workers to use for zahirscan. Sequential mode: all available; else override or ratio share.
    pub fn effective_zahir_workers(&self) -> usize {
        let (_, z, _) = self.default_share();
        self.effective_workers_for(self.zahir_workers_override, z)
    }

    /// Workers reserved for ublx (main process / other work). Sequential mode: all available; else override or remainder.
    #[allow(dead_code)]
    pub fn effective_ublx_workers(&self) -> usize {
        let (_, _, u) = self.default_share();
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

    /// ZahirScan runtime config with ublx overrides: [OutputMode::Templates], [Self::effective_zahir_workers] for `max_workers`.
    #[allow(dead_code)]
    pub fn zahir_runtime_config(&self) -> RuntimeConfig {
        let mut config = self.zahir.clone();
        // config.output_mode = OutputMode::Full;
        config.output_mode = OutputMode::Templates;
        config.max_workers = self.effective_zahir_workers();
        config
    }
}

/// Write local config with `theme = "display_name"`. Uses existing file at [UblxPaths::toml_path] if present, otherwise creates `.ublx.toml`. Preserves other keys from existing file or default. Logs and ignores errors.
pub fn write_local_theme(paths: &UblxPaths, theme_display_name: &str) {
    let path = paths.toml_path().unwrap_or_else(|| paths.hidden_toml());
    let mut overlay = UblxOpts::load_overlay_from_path(Some(path.clone()));
    overlay.theme = Some(theme_display_name.to_string());
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        warn!("could not create config dir {}: {}", parent.display(), e);
        return;
    }
    match toml::to_string_pretty(&overlay) {
        Ok(s) => {
            if let Err(e) = fs::write(&path, s) {
                warn!("could not write theme to {}: {}", path.display(), e);
            }
        }
        Err(e) => warn!("could not serialize overlay: {}", e),
    }
}
