//! Load/save [`UblxOverlay`] from TOML paths and config cache.

use std::fs;
use std::path::Path;

use log::warn;

use super::{EnhancePolicy, EnhancePolicyEntry, UblxOverlay};

use crate::config::paths::{UblxPaths, normalize_rel_path_for_policy};
use crate::config::theme::auto_correct_theme_name;

#[must_use]
pub fn load_ublx_toml(
    path: Option<std::path::PathBuf>,
    valid_theme_names: Option<&[&str]>,
) -> Option<UblxOverlay> {
    let path = path?;
    let s = fs::read_to_string(&path).ok()?;
    match toml::from_str::<UblxOverlay>(&s) {
        Ok(mut overlay) => {
            let corrected_theme =
                valid_theme_names
                    .zip(overlay.theme.as_deref())
                    .and_then(|(valid, theme)| {
                        auto_correct_theme_name(theme, valid)
                            .filter(|corrected| *corrected != theme)
                    });
            if let Some(corrected) = corrected_theme {
                overlay.theme = Some(corrected.to_string());
                write_corrected_overlay(&path, &overlay);
            }
            Some(overlay)
        }
        Err(e) => {
            warn!("{}: parse error, ignoring: {}", path.display(), e);
            None
        }
    }
}

pub fn write_corrected_overlay(path: &Path, overlay: &UblxOverlay) {
    match toml::to_string_pretty(overlay) {
        Ok(updated) => {
            if let Err(e) = fs::write(path, updated) {
                warn!(
                    "could not write corrected theme to {}: {}",
                    path.display(),
                    e
                );
            }
        }
        Err(e) => warn!(
            "could not serialize corrected overlay {}: {}",
            path.display(),
            e
        ),
    }
}

pub fn save_overlay_to_cache(ublx_paths: &UblxPaths, overlay: &UblxOverlay) {
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

/// Write local config with `theme = "display_name"`. Uses existing file at [`UblxPaths::toml_path`] if present, otherwise creates `.ublx.toml`. Preserves other keys from existing file or default. Logs and ignores errors.
pub fn write_local_theme(paths: &UblxPaths, theme_display_name: &str) {
    let path = paths.toml_path().unwrap_or_else(|| paths.hidden_toml());
    let mut overlay = load_ublx_toml(Some(path.clone()), None).unwrap_or_default();
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
    let mut overlay = load_ublx_toml(Some(path.clone()), None).unwrap_or_default();
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
