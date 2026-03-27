//! Settings bool rows: global vs local `UblxOverlay` flags (`show_hidden_files`, `hash`, etc.).

use crate::config::UblxOverlay;
use crate::layout::setup::SettingsConfigScope;

/// Maps Settings left-pane row index → [`crate::config::UblxOverlay`] bool field. Local scope uses rows 0–2; Global adds
/// row 3 (`ask_enhance_on_new_root`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsBoolKey {
    ShowHiddenFiles,
    Hash,
    EnableEnhanceAll,
    AskEnhanceOnNewRoot,
}

#[must_use]
pub fn bool_key(scope: SettingsConfigScope, idx: usize) -> Option<SettingsBoolKey> {
    match scope {
        SettingsConfigScope::Global => match idx {
            0 => Some(SettingsBoolKey::ShowHiddenFiles),
            1 => Some(SettingsBoolKey::Hash),
            2 => Some(SettingsBoolKey::EnableEnhanceAll),
            3 => Some(SettingsBoolKey::AskEnhanceOnNewRoot),
            _ => None,
        },
        SettingsConfigScope::Local => match idx {
            0 => Some(SettingsBoolKey::ShowHiddenFiles),
            1 => Some(SettingsBoolKey::Hash),
            2 => Some(SettingsBoolKey::EnableEnhanceAll),
            _ => None,
        },
    }
}

/// `true` if this key is present in the local file (so it is not inherited-only).
#[must_use]
pub fn local_bool_is_explicit(local: Option<&UblxOverlay>, idx: usize) -> bool {
    let Some(l) = local else {
        return false;
    };
    let Some(key) = bool_key(SettingsConfigScope::Local, idx) else {
        return false;
    };
    match key {
        SettingsBoolKey::ShowHiddenFiles => l.show_hidden_files.is_some(),
        SettingsBoolKey::Hash => l.hash.is_some(),
        SettingsBoolKey::EnableEnhanceAll => l.enable_enhance_all.is_some(),
        SettingsBoolKey::AskEnhanceOnNewRoot => false,
    }
}

/// Number of bool rows for the active scope (global includes `ask_enhance_on_new_root`).
#[must_use]
pub fn bool_row_count(scope: SettingsConfigScope) -> usize {
    match scope {
        SettingsConfigScope::Global => 4,
        SettingsConfigScope::Local => 3,
    }
}

#[must_use]
pub fn bool_row_label(scope: SettingsConfigScope, idx: usize) -> &'static str {
    bool_key(scope, idx).map_or("?", |key| match key {
        SettingsBoolKey::ShowHiddenFiles => "show_hidden_files",
        SettingsBoolKey::Hash => "hash",
        SettingsBoolKey::EnableEnhanceAll => "enable_enhance_all",
        SettingsBoolKey::AskEnhanceOnNewRoot => "ask_enhance_on_new_root",
    })
}

#[must_use]
pub fn overlay_bool(overlay: &UblxOverlay, scope: SettingsConfigScope, idx: usize) -> bool {
    let Some(key) = bool_key(scope, idx) else {
        return false;
    };
    match key {
        SettingsBoolKey::ShowHiddenFiles => overlay.show_hidden_files.unwrap_or(false),
        SettingsBoolKey::Hash => overlay.hash.unwrap_or(false),
        SettingsBoolKey::EnableEnhanceAll => overlay.enable_enhance_all.unwrap_or(false),
        SettingsBoolKey::AskEnhanceOnNewRoot => overlay.ask_enhance_on_new_root.unwrap_or(true),
    }
}

pub fn write_bool(overlay: &mut UblxOverlay, scope: SettingsConfigScope, idx: usize, v: bool) {
    let Some(key) = bool_key(scope, idx) else {
        return;
    };
    match key {
        SettingsBoolKey::ShowHiddenFiles => overlay.show_hidden_files = Some(v),
        SettingsBoolKey::Hash => overlay.hash = Some(v),
        SettingsBoolKey::EnableEnhanceAll => overlay.enable_enhance_all = Some(v),
        SettingsBoolKey::AskEnhanceOnNewRoot => overlay.ask_enhance_on_new_root = Some(v),
    }
}
