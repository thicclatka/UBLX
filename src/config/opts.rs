//! Options for ublx, extending per-tool opts (e.g. `NefaxOpts`, zahirscan `RuntimeConfig`).
//!
//! Worker pool: [`Self::max_workers_available`] is derived from nefax tuning (drive-type aware).
//! When >= [`Self::SEQUENTIAL_THRESHOLD`], workers are split by ratio (or overrides) across nefax, zahir, and ublx.
//! When below threshold, sequential mode: run phases one after another, each using all available workers.
//! For zahir-only (e.g. single file, no nefax), use [`Self::for_zahir_only`] with a chosen max (e.g. from tuning).
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
use super::toast::OPERATION_NAME;
use super::validation::{
    HotReloadValidationError, ReloadResult, first_validation_error_message,
    validate_hot_reload_overlay,
};
use crate::handlers::nefax_ops::{NefaxDriveType, pre_opts_for_nefaxer};
use crate::utils::notifications::BumperBuffer;

/// Parameters for config validation and optional bumper when loading opts in [`UblxOpts::for_dir`].
pub struct ForDirConfig<'a> {
    pub valid_theme_names: &'a [&'a str],
    pub bumper: Option<&'a BumperBuffer>,
}

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
#[must_use]
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

/// At or above this many workers we set [`UblxOpts::streaming`] to true (callback path for nefax).
pub const STREAMING_THRESHOLD: usize = 6;

/// Layout pane percentages (0–100). Used for main 3-pane split: left (categories), middle (contents), right (preview).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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

/// Per-directory policy for automatic (index-time) `ZahirScan`. Longest matching path prefix wins; absent entry inherits [`UblxOpts::enable_enhance_all`].
/// Does not apply to per-file "Enhance with `ZahirScan`" from the space menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnhancePolicy {
    /// Index-time batch Zahir for paths under this subtree (same idea as global `enable_enhance_all` for that prefix).
    #[serde(alias = "always")]
    Auto,
    /// No batch Zahir under this subtree; enrich per file from the space menu only.
    #[serde(alias = "never")]
    Manual,
}

/// One `[[enhance_policy]]` row in `ublx.toml` / `.ublx.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct EnhancePolicyEntry {
    /// Path relative to the indexed directory, using `/` separators (e.g. `photos/vacation`).
    pub path: String,
    pub policy: EnhancePolicy,
}

/// Config overlay read from config files. Only present keys override; used for global + local overlay.
/// Apply in order: defaults → global `~/.config/ublx/ublx.toml` → local `.ublx.toml` or `ublx.toml` in indexed dir.
/// [theme], [transparent], [layout], [hash], and [`show_hidden_files`] are hot-reloadable; [exclude] is applied only at startup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct UblxOverlay {
    /// Extra paths/patterns to exclude from indexing (appended to nefax [`NefaxOpts::exclude`]). Startup-only; not hot-reloadable.
    pub exclude: Option<Vec<String>>,
    /// When true, show hidden files; when false, exclude `.*` per segment and zahir skips hidden. Hot-reloadable.
    /// When omitted, treated as false (same as explicit false).
    #[serde(rename = "show_hidden_files")]
    pub show_hidden_files: Option<bool>,
    /// When true, nefaxer computes blake3 hash for files (slower, more accurate change detection). Hot-reloadable.
    pub hash: Option<bool>,
    /// Theme selection (e.g. "default"). Hot-reloadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// When true, do not paint app background; terminal default (or transparency) shows through. Hot-reloadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent: Option<bool>,
    /// Optional [layout] section: left/middle/right pane percentages. Hot-reloadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutOverlay>,
    /// Editor for Open (Terminal) (e.g. "vim", "nvim"). When unset, uses $EDITOR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_path: Option<String>,
    /// When `true`, index runs full `ZahirScan` enrichment on paths that need it (normal pipeline).
    /// When `false` (default), only nefax + path-based category from `ZahirScan` file-type hints; empty `zahir_json` until per-file "Enhance with `ZahirScan`" or flip to `true` (next run re-enhances all). Hot-reloadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_enhance_all: Option<bool>,
    /// Optional per-path subtree rules for index-time Zahir (`[[enhance_policy]]`). Hot-reloadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhance_policy: Option<Vec<EnhancePolicyEntry>>,
}

impl UblxOverlay {
    /// Overlay with values from `other`; only fields set in `other` are applied (local overrides global when merging).
    pub fn merge_from(&mut self, other: &UblxOverlay) {
        if other.exclude.is_some() {
            self.exclude.clone_from(&other.exclude);
        }
        if other.show_hidden_files.is_some() {
            self.show_hidden_files = other.show_hidden_files;
        }
        if other.hash.is_some() {
            self.hash = other.hash;
        }
        if other.theme.is_some() {
            self.theme.clone_from(&other.theme);
        }
        if other.transparent.is_some() {
            self.transparent = other.transparent;
        }
        if other.layout.is_some() {
            self.layout.clone_from(&other.layout);
        }
        if other.editor_path.is_some() {
            self.editor_path.clone_from(&other.editor_path);
        }
        if other.enable_enhance_all.is_some() {
            self.enable_enhance_all = other.enable_enhance_all;
        }
        if other.enhance_policy.is_some() {
            self.enhance_policy.clone_from(&other.enhance_policy);
        }
    }

    /// Merge global then local into one overlay (local wins). Used to apply once and to cache the effective config.
    #[must_use]
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

/// Options for ublx. Extends [`NefaxOpts`] and [`RuntimeConfig`]; owns worker-pool sizing and streaming.
#[derive(Clone, Debug)]
pub struct UblxOpts {
    /// Options passed to the nefaxer indexer (base; use [`Self::nefax_opts_with_workers`] for run).
    pub nefax: NefaxOpts,
    /// `ZahirScan` runtime config. Use [`Self::zahir_runtime_config`] to get config with ublx overrides applied.
    pub zahir: RuntimeConfig,
    /// Max workers suggested by tuning (drive-type aware). From nefax [`tuning_for_path`] in [`Self::for_dir`].
    pub max_workers_available: usize,
    /// Override workers for nefaxer. When unset and >= [`SEQUENTIAL_THRESHOLD`], uses share from 1:1:1 ratio.
    pub nefax_workers_override: Option<usize>,
    /// Override workers for zahirscan. When unset and >= threshold, uses share from ratio.
    pub zahir_workers_override: Option<usize>,
    /// Override workers for ublx (main process / other work). When unset and >= threshold, remainder from ratio.
    #[allow(dead_code)]
    pub ublx_workers_override: Option<usize>,
    /// Use streaming (callback) path for nefax when true.
    pub streaming: bool,
    /// When global config exists: "local" | "global". Preserved from `cached_settings` for writing back to DB.
    pub config_source: Option<String>,
    /// Theme name (e.g. "default"). From config overlay; used by `layout::themes::get`.
    pub theme: Option<String>,
    /// When true, skip painting app background so terminal default/transparency shows.
    pub transparent: bool,
    /// Left/middle/right pane percentages (0–100). From config [layout]. Hot-reloadable.
    pub layout: LayoutOverlay,
    /// Editor for Open (Terminal). When None, use $EDITOR.
    pub editor_path: Option<String>,
    /// When true, run full `ZahirScan` on indexed files; when false, path-only category + space-menu enhance.
    pub enable_enhance_all: bool,
    /// `enable_enhance_all` from the config cache **before** [`Self::for_dir`] applied the current overlay and called [`Self::save_overlay_to_cache`]. Used by snapshot `force_full` Zahir when flipping the flag to `true`.
    pub enable_enhance_all_cache_before_apply: Option<bool>,
    /// Effective `[[enhance_policy]]` entries (merged global + local). Used only for index-time batch Zahir.
    pub enhance_policy: Vec<EnhancePolicyEntry>,
}

impl UblxOpts {
    /// Build ublx options for indexing `dir`. [`Self::max_workers_available`] comes from [`tuning_for_path`](nefaxer::tuning_for_path).
    /// Zahir config is loaded with [`RuntimeConfig::new`]. [`Self::streaming`] is set true when workers >= [`STREAMING_THRESHOLD`].
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

    /// Load the last applied overlay from cache (`cache_dir()/configs/[path_hex].toml`). Fallback when hot reload gets invalid config.
    #[must_use]
    pub fn load_overlay_from_cache(ublx_paths: &UblxPaths) -> Option<UblxOverlay> {
        Self::load_ublx_toml(ublx_paths.last_applied_config_path())
    }

    /// Load merged overlay (global then local). Same merge as [`Self::for_dir`]; used for hot reload.
    #[must_use]
    pub fn load_merged_overlay(ublx_paths: &UblxPaths) -> UblxOverlay {
        let global = Self::load_ublx_toml(ublx_paths.global_config());
        let local = Self::load_ublx_toml(ublx_paths.toml_path());
        UblxOverlay::merge(global, local)
    }

    /// Load overlay from a single toml file path. Returns default if path is None, file missing, or parse error.
    #[must_use]
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
            Err(e) => warn!("could not serialize overlay for cache: {e}"),
        }
    }

    /// Apply full overlay at startup (exclude + hot-reloadable fields). [exclude] is only applied here.
    fn apply_overlay(&mut self, overlay: &UblxOverlay) {
        if let Some(ref extra) = overlay.exclude {
            self.nefax.exclude.extend(extra.iter().cloned());
        }
        self.apply_hot_reload_overlay(overlay);
    }

    /// Apply only hot-reloadable fields: theme, transparent, layout, hash, `show_hidden_files`. Used when reloading config without restart.
    /// On invalid config from disk, caller should fall back to [`Self::load_overlay_from_cache`] and pass that overlay here.
    pub fn apply_hot_reload_overlay(&mut self, overlay: &UblxOverlay) {
        // When `show_hidden_files` is omitted from merged overlay, default to false (do not index dotfiles).
        // Otherwise `.*` is never added to nefax exclude and paths like `.mise.toml` are still walked.
        let show_hidden = overlay.show_hidden_files.unwrap_or(false);
        if show_hidden {
            self.nefax.exclude.retain(|p| p != HIDDEN_EXCLUDE_PATTERN);
            self.zahir.flags.ignore_hidden_files = false;
        } else {
            if !self
                .nefax
                .exclude
                .iter()
                .any(|p| p == HIDDEN_EXCLUDE_PATTERN)
            {
                self.nefax.exclude.push(HIDDEN_EXCLUDE_PATTERN.to_string());
            }
            self.zahir.flags.ignore_hidden_files = true;
        }
        if let Some(hash) = overlay.hash {
            self.nefax.with_hash = hash;
        }
        if overlay.theme.is_some() {
            self.theme.clone_from(&overlay.theme);
        }
        if overlay.transparent.is_some() {
            self.transparent = overlay.transparent.unwrap_or(false);
        }
        if overlay.layout.is_some() {
            self.layout = overlay.layout.clone().unwrap_or_default();
        }
        if overlay.editor_path.is_some() {
            self.editor_path.clone_from(&overlay.editor_path);
        }
        if let Some(v) = overlay.enable_enhance_all {
            self.enable_enhance_all = v;
        }
        self.enhance_policy = overlay.enhance_policy.clone().unwrap_or_default();
    }

    /// Reload hot-reloadable config from disk (global + local merge). When disk yields no config, falls back to cached overlay from last successful load.
    /// On success, writes the merged overlay to the config cache (same path as startup `for_dir`).
    /// Validates before applying; on validation failure the overlay is not applied and errors are returned.
    /// `valid_theme_names`: allowed theme values (e.g. from [`crate::layout::themes::theme_ordered_list`] names).
    pub fn reload_hot_config(
        &mut self,
        ublx_paths: &UblxPaths,
        valid_theme_names: &[&str],
    ) -> ReloadResult {
        let from_disk = Self::load_merged_overlay(ublx_paths);
        let to_apply = if from_disk == UblxOverlay::default() {
            Self::load_overlay_from_cache(ublx_paths).unwrap_or_default()
        } else {
            from_disk
        };
        if to_apply == UblxOverlay::default() {
            return ReloadResult::default();
        }
        match validate_hot_reload_overlay(&to_apply, valid_theme_names) {
            Ok(()) => {
                self.apply_hot_reload_overlay(&to_apply);
                Self::save_overlay_to_cache(ublx_paths, &to_apply);
                ReloadResult {
                    applied: true,
                    validation_errors: Vec::new(),
                }
            }
            Err(validation_errors) => {
                let cached = Self::load_overlay_from_cache(ublx_paths).unwrap_or_default();
                self.apply_hot_reload_overlay(&cached);
                ReloadResult {
                    applied: false,
                    validation_errors,
                }
            }
        }
    }

    /// Message for bumper when startup config is invalid and we fall back to cache.
    fn startup_validation_fallback_message(errors: &[HotReloadValidationError]) -> String {
        format!(
            "Config invalid at startup, using cache: {}",
            first_validation_error_message(errors)
        )
    }

    /// Build ublx options for indexing `dir`. When `cached_settings` is `Some`, use those values and skip disk check; otherwise call [`tuning_for_path`](nefaxer::tuning_for_path).
    /// Zahir config is loaded with [`RuntimeConfig::new`]. [`Self::streaming`] is set true when workers >= [`STREAMING_THRESHOLD`].
    /// If a config file exists (`paths.toml_path()`: `.ublx.toml` or `ublx.toml`), only keys present in it overlay these opts.
    /// Merged overlay is validated; if invalid, cache (or default) is applied and not saved.
    /// When validation fails, if `config.bumper` is provided, pushes a warning so the user sees "using cache" (e.g. toast in TUI).
    #[must_use]
    pub fn for_dir(
        dir_to_ublx: &Path,
        ublx_paths: &UblxPaths,
        nefax_workers_override: Option<usize>,
        zahir_workers_override: Option<usize>,
        ublx_workers_override: Option<usize>,
        cached_settings: Option<&UblxSettings>,
        config: &ForDirConfig<'_>,
    ) -> Self {
        let exclude = ublx_paths.exclude();
        let nefax = pre_opts_for_nefaxer(dir_to_ublx, &exclude, cached_settings);
        let num_threads = nefax.num_threads.unwrap_or(1);
        let zahir = RuntimeConfig::new();
        let streaming = num_threads >= STREAMING_THRESHOLD;
        let config_source = cached_settings.and_then(|s| s.config_source.clone());
        let enable_enhance_all_cache_before_apply =
            Self::load_overlay_from_cache(ublx_paths).and_then(|o| o.enable_enhance_all);
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
            editor_path: None,
            enable_enhance_all: false,
            enable_enhance_all_cache_before_apply,
            enhance_policy: Vec::new(),
        };
        let global = Self::load_ublx_toml(ublx_paths.global_config());
        let local = Self::load_ublx_toml(ublx_paths.toml_path());
        let merged = UblxOverlay::merge(global, local);
        match validate_hot_reload_overlay(&merged, config.valid_theme_names) {
            Ok(()) => {
                opts.apply_overlay(&merged);
                Self::save_overlay_to_cache(ublx_paths, &merged);
            }
            Err(validation_errors) => {
                let cached = Self::load_overlay_from_cache(ublx_paths).unwrap_or_default();
                opts.apply_overlay(&cached);
                if let Some(b) = config.bumper {
                    b.push_with_operation(
                        log::Level::Warn,
                        Self::startup_validation_fallback_message(&validation_errors).as_str(),
                        Some(OPERATION_NAME.op("settings")),
                    );
                }
            }
        }
        opts
    }

    /// Build [`UblxSettings`] from this opts for writing to the ublx DB (so next run can skip disk check).
    pub fn to_ublx_settings(&self) -> UblxSettings {
        UblxSettings {
            num_threads: self.max_workers_available,
            drive_type: self
                .nefax
                .drive_type
                .map_or("Unknown", drive_type_to_string)
                .to_string(),
            parallel_walk: self.nefax.use_parallel_walk.unwrap_or(false),
            config_source: self.config_source.clone(),
        }
    }

    /// Build opts when running zahir only (e.g. single file, no nefax). You supply [`Self::max_workers_available`] (e.g. from tuning on a path); all are used for zahir.
    /// [`Self::streaming`] is set true when workers >= [`STREAMING_THRESHOLD`].
    #[allow(dead_code)]
    #[must_use]
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
            editor_path: None,
            enable_enhance_all: true,
            enable_enhance_all_cache_before_apply: None,
            enhance_policy: Vec::new(),
        }
    }

    /// True when [`Self::max_workers_available`] < [`SEQUENTIAL_THRESHOLD`]: run phases sequentially, each phase using all workers.
    #[must_use]
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

    /// Workers to use for nefaxer. Sequential mode: all [`Self::max_workers_available`]; else override or ratio share.
    #[must_use]
    pub fn effective_nefax_workers(&self) -> usize {
        let (n, _, _) = self.default_share();
        self.effective_workers_for(self.nefax_workers_override, n)
    }

    /// Workers to use for zahirscan. Sequential mode: all available; else override or ratio share.
    #[must_use]
    pub fn effective_zahir_workers(&self) -> usize {
        let (_, z, _) = self.default_share();
        self.effective_workers_for(self.zahir_workers_override, z)
    }

    /// Workers reserved for ublx (main process / other work). Sequential mode: all available; else override or remainder.
    #[allow(dead_code)]
    #[must_use]
    pub fn effective_ublx_workers(&self) -> usize {
        let (_, _, u) = self.default_share();
        self.effective_workers_for(self.ublx_workers_override, u)
    }

    /// Reference to the inner [`NefaxOpts`] (base, no worker override applied).
    #[allow(dead_code)]
    #[must_use]
    pub fn nefax_opts(&self) -> &NefaxOpts {
        &self.nefax
    }

    /// [`NefaxOpts`] with [`Self::effective_nefax_workers`] applied to `num_threads` for use with [`nefax_ops::run_nefaxer`].
    #[allow(dead_code)]
    #[must_use]
    pub fn nefax_opts_with_workers(&self) -> NefaxOpts {
        let mut opts = self.nefax.clone();
        opts.num_threads = Some(self.effective_nefax_workers());
        opts
    }

    /// `ZahirScan` runtime config with ublx overrides: [`OutputMode::Templates`], [`Self::effective_zahir_workers`] for `max_workers`.
    #[allow(dead_code)]
    #[must_use]
    pub fn zahir_runtime_config(&self) -> RuntimeConfig {
        let mut config = self.zahir.clone();
        // config.output_mode = OutputMode::Full;
        config.output_mode = OutputMode::Templates;
        config.max_workers = self.effective_zahir_workers();
        config
    }

    /// Whether index-time batch `ZahirScan` should run for this relative path (longest `[[enhance_policy]]` prefix wins; else [`Self::enable_enhance_all`]).
    #[must_use]
    pub fn batch_zahir_for_path(&self, rel_path: &str) -> bool {
        let rel = normalize_rel_path_for_policy(rel_path);
        let mut best: Option<(usize, EnhancePolicy)> = None;
        for e in &self.enhance_policy {
            let p = normalize_rel_path_for_policy(&e.path);
            if p.is_empty() {
                continue;
            }
            if path_is_under_or_equal(&rel, &p) {
                let len = p.len();
                if best.as_ref().is_none_or(|(blen, _)| len > *blen) {
                    best = Some((len, e.policy));
                }
            }
        }
        match best.map(|(_, pol)| pol) {
            Some(EnhancePolicy::Auto) => true,
            Some(EnhancePolicy::Manual) => false,
            None => self.enable_enhance_all,
        }
    }
}

#[must_use]
fn normalize_rel_path_for_policy(s: &str) -> String {
    let s = s.replace('\\', "/");
    let s = s.trim_start_matches("./");
    s.trim_end_matches('/').to_string()
}

fn path_is_under_or_equal(rel: &str, prefix: &str) -> bool {
    rel == prefix || (rel.starts_with(prefix) && rel.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Write local config with `theme = "display_name"`. Uses existing file at [`UblxPaths::toml_path`] if present, otherwise creates `.ublx.toml`. Preserves other keys from existing file or default. Logs and ignores errors.
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
        Err(e) => warn!("could not serialize overlay: {e}"),
    }
}

/// Write `ublx.toml` in the indexed directory with only [`UblxOverlay::enable_enhance_all`] set (first-run prompt).
///
/// # Errors
///
/// Returns [`std::io::Error`] if TOML serialization fails or if writing [`UblxPaths::visible_toml`] fails
/// (e.g. missing parent directory, permission denied, or disk full).
pub fn write_visible_enhance_only_toml(
    ublx_paths: &UblxPaths,
    enable_enhance_all: bool,
) -> std::io::Result<()> {
    let overlay = UblxOverlay {
        enable_enhance_all: Some(enable_enhance_all),
        ..Default::default()
    };
    let s = toml::to_string(&overlay).map_err(std::io::Error::other)?;
    fs::write(ublx_paths.visible_toml(), s)
}

/// Merge `[[enhance_policy]]` for `rel_path` into local config (`.ublx.toml` or `ublx.toml`). Preserves other keys.
pub fn write_local_enhance_policy(paths: &UblxPaths, rel_path: &str, policy: EnhancePolicy) {
    let path = paths.toml_path().unwrap_or_else(|| paths.hidden_toml());
    let mut overlay = UblxOpts::load_overlay_from_path(Some(path.clone()));
    let mut entries = overlay.enhance_policy.unwrap_or_default();
    let norm = normalize_rel_path_for_policy(rel_path);
    entries.retain(|e| normalize_rel_path_for_policy(&e.path) != norm);
    entries.push(EnhancePolicyEntry { path: norm, policy });
    overlay.enhance_policy = Some(entries);
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        warn!("could not create config dir {}: {}", parent.display(), e);
        return;
    }
    match toml::to_string_pretty(&overlay) {
        Ok(s) => {
            if let Err(e) = fs::write(&path, s) {
                warn!(
                    "could not write enhance_policy to {}: {}",
                    path.display(),
                    e
                );
            }
        }
        Err(e) => warn!("could not serialize overlay: {e}"),
    }
}
