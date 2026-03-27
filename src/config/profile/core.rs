//! Config overlay types: TOML `[layout]`, `theme`, `[[enhance_policy]]`, etc.

use serde::{Deserialize, Serialize};

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
            left_pct: 10,
            middle_pct: 30,
            right_pct: 60,
        }
    }
}

/// Per-directory policy for automatic (index-time) `ZahirScan`. Longest matching path prefix wins; absent entry inherits [`crate::config::UblxOpts::enable_enhance_all`].
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
/// [theme], [layout], [hash], and [`show_hidden_files`] are hot-reloadable; [exclude] is applied only at startup.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct UblxOverlay {
    /// Extra paths/patterns to exclude from indexing (appended to nefax [`nefaxer::NefaxOpts::exclude`]). Startup-only; not hot-reloadable.
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
    /// When `false`, skip the first-run "Enhance all files" prompt for a new root; use [`Self::enable_enhance_all`] from config instead. Set in `~/.config/ublx/ublx.toml` to stop being asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_enhance_on_new_root: Option<bool>,
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
        if other.layout.is_some() {
            self.layout.clone_from(&other.layout);
        }
        if other.editor_path.is_some() {
            self.editor_path.clone_from(&other.editor_path);
        }
        if other.enable_enhance_all.is_some() {
            self.enable_enhance_all = other.enable_enhance_all;
        }
        if other.ask_enhance_on_new_root.is_some() {
            self.ask_enhance_on_new_root = other.ask_enhance_on_new_root;
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
