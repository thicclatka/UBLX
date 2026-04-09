//! Settings bool rows: global vs local `UblxOverlay` flags (`show_hidden_files`, `hash`, etc.).

use std::borrow::Cow;

use crate::config::UblxOverlay;
use crate::layout::setup::SettingsConfigScope;
use crate::ui::UI_STRINGS;

/// Maps Settings left-pane row index → [`crate::config::UblxOverlay`] bool field. Global row 4 is
/// `run_snapshot_on_startup` (after `ask_enhance_on_new_root`); local row 3 is `run_snapshot_on_startup` (no `ask_enhance` row).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsBoolKey {
    ShowHiddenFiles,
    Hash,
    EnableEnhanceAll,
    AskEnhanceOnNewRoot,
    RunSnapshotOnStartup,
}

#[must_use]
pub fn bool_key(scope: SettingsConfigScope, idx: usize) -> Option<SettingsBoolKey> {
    match scope {
        SettingsConfigScope::Global => match idx {
            0 => Some(SettingsBoolKey::ShowHiddenFiles),
            1 => Some(SettingsBoolKey::Hash),
            2 => Some(SettingsBoolKey::EnableEnhanceAll),
            3 => Some(SettingsBoolKey::AskEnhanceOnNewRoot),
            4 => Some(SettingsBoolKey::RunSnapshotOnStartup),
            _ => None,
        },
        SettingsConfigScope::Local => match idx {
            0 => Some(SettingsBoolKey::ShowHiddenFiles),
            1 => Some(SettingsBoolKey::Hash),
            2 => Some(SettingsBoolKey::EnableEnhanceAll),
            3 => Some(SettingsBoolKey::RunSnapshotOnStartup),
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
        SettingsBoolKey::RunSnapshotOnStartup => l.run_snapshot_on_startup.is_some(),
    }
}

/// Number of bool rows for the active scope (global: `ask_enhance_on_new_root` then `run_snapshot_on_startup`).
#[must_use]
pub fn bool_row_count(scope: SettingsConfigScope) -> usize {
    match scope {
        SettingsConfigScope::Global => 5,
        SettingsConfigScope::Local => 4,
    }
}

/// Row label text (bare TOML key). The left pane draws [`crate::ui::SETTINGS_BOOL_SNAPSHOT_STAR_PREFIX`] before snapshot-affecting keys when the row is inactive.
#[must_use]
pub fn bool_row_label(
    scope: SettingsConfigScope,
    idx: usize,
    _for_left_pane: bool,
) -> Cow<'static, str> {
    let l = &UI_STRINGS.settings_bool;
    bool_key(scope, idx).map_or(Cow::Borrowed(l.unknown_row), |key| {
        let base = match key {
            SettingsBoolKey::ShowHiddenFiles => l.show_hidden_files,
            SettingsBoolKey::Hash => l.hash,
            SettingsBoolKey::EnableEnhanceAll => l.enable_enhance_all,
            SettingsBoolKey::AskEnhanceOnNewRoot => l.ask_enhance_on_new_root,
            SettingsBoolKey::RunSnapshotOnStartup => l.run_snapshot_on_startup,
        };
        Cow::Borrowed(base)
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
        SettingsBoolKey::RunSnapshotOnStartup => overlay.run_snapshot_on_startup.unwrap_or(true),
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
        SettingsBoolKey::RunSnapshotOnStartup => overlay.run_snapshot_on_startup = Some(v),
    }
}
