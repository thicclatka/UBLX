//! Settings bool rows: global vs local `UblxOverlay` flags (`show_hidden_files`, `hash`, etc.).

use std::borrow::Cow;

use crate::config::UblxOverlay;
use crate::layout::setup::SettingsConfigScope;
use crate::ui::UI_STRINGS;

/// Maps Settings left-pane row index → [`crate::config::UblxOverlay`] bool field. Global row 5 is
/// `run_snapshot_on_startup` (after `ask_enhance_on_new_root`); local row 4 is `run_snapshot_on_startup` (no `ask_enhance` row).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsBoolKey {
    ShowHiddenFiles,
    Hash,
    FollowLinks,
    EnableEnhanceAll,
    AskEnhanceOnNewRoot,
    RunSnapshotOnStartup,
}

/// Shared Global/Local prefix; Global then adds `ask_enhance_on_new_root`, then both end with `run_snapshot_on_startup`.
const SHARED_BOOL_KEYS: [SettingsBoolKey; 4] = [
    SettingsBoolKey::ShowHiddenFiles,
    SettingsBoolKey::Hash,
    SettingsBoolKey::FollowLinks,
    SettingsBoolKey::EnableEnhanceAll,
];

impl SettingsBoolKey {
    /// TOML / API key name (also the Settings left-pane label).
    #[must_use]
    pub const fn toml_key(self) -> &'static str {
        match self {
            Self::ShowHiddenFiles => "show_hidden_files",
            Self::Hash => "hash",
            Self::FollowLinks => "follow_links",
            Self::EnableEnhanceAll => "enable_enhance_all",
            Self::AskEnhanceOnNewRoot => "ask_enhance_on_new_root",
            Self::RunSnapshotOnStartup => "run_snapshot_on_startup",
        }
    }

    /// Keys marked with `*` in Settings — applied on the next snapshot, not immediately.
    #[must_use]
    pub const fn affects_next_snapshot(self) -> bool {
        matches!(
            self,
            Self::ShowHiddenFiles | Self::Hash | Self::FollowLinks | Self::EnableEnhanceAll
        )
    }

    /// Effective value when the overlay omits this key.
    #[must_use]
    pub const fn default_when_missing(self) -> bool {
        match self {
            Self::AskEnhanceOnNewRoot | Self::RunSnapshotOnStartup => true,
            Self::ShowHiddenFiles | Self::Hash | Self::FollowLinks | Self::EnableEnhanceAll => {
                false
            }
        }
    }

    fn overlay_opt(self, overlay: &UblxOverlay) -> Option<bool> {
        match self {
            Self::ShowHiddenFiles => overlay.show_hidden_files,
            Self::Hash => overlay.hash,
            Self::FollowLinks => overlay.follow_links,
            Self::EnableEnhanceAll => overlay.enable_enhance_all,
            Self::AskEnhanceOnNewRoot => overlay.ask_enhance_on_new_root,
            Self::RunSnapshotOnStartup => overlay.run_snapshot_on_startup,
        }
    }

    fn write(self, overlay: &mut UblxOverlay, v: bool) {
        match self {
            Self::ShowHiddenFiles => overlay.show_hidden_files = Some(v),
            Self::Hash => overlay.hash = Some(v),
            Self::FollowLinks => overlay.follow_links = Some(v),
            Self::EnableEnhanceAll => overlay.enable_enhance_all = Some(v),
            Self::AskEnhanceOnNewRoot => overlay.ask_enhance_on_new_root = Some(v),
            Self::RunSnapshotOnStartup => overlay.run_snapshot_on_startup = Some(v),
        }
    }
}

#[must_use]
pub fn bool_key(scope: SettingsConfigScope, idx: usize) -> Option<SettingsBoolKey> {
    if let Some(&key) = SHARED_BOOL_KEYS.get(idx) {
        return Some(key);
    }
    match scope {
        SettingsConfigScope::Global => match idx {
            4 => Some(SettingsBoolKey::AskEnhanceOnNewRoot),
            5 => Some(SettingsBoolKey::RunSnapshotOnStartup),
            _ => None,
        },
        SettingsConfigScope::Local => {
            (idx == SHARED_BOOL_KEYS.len()).then_some(SettingsBoolKey::RunSnapshotOnStartup)
        }
    }
}

/// `true` if this key is present in the local file (so it is not inherited-only).
#[must_use]
pub fn local_bool_is_explicit(local: Option<&UblxOverlay>, idx: usize) -> bool {
    let Some(l) = local else {
        return false;
    };
    bool_key(SettingsConfigScope::Local, idx).is_some_and(|key| key.overlay_opt(l).is_some())
}

/// Number of bool rows for the active scope (global: `ask_enhance_on_new_root` then `run_snapshot_on_startup`).
#[must_use]
pub fn bool_row_count(scope: SettingsConfigScope) -> usize {
    match scope {
        SettingsConfigScope::Global => SHARED_BOOL_KEYS.len() + 2,
        SettingsConfigScope::Local => SHARED_BOOL_KEYS.len() + 1,
    }
}

/// Row label text (bare TOML key). The left pane draws a snapshot `*` before
/// [`SettingsBoolKey::affects_next_snapshot`] keys when the row is inactive.
#[must_use]
pub fn bool_row_label(
    scope: SettingsConfigScope,
    idx: usize,
    _for_left_pane: bool,
) -> Cow<'static, str> {
    bool_key(scope, idx).map_or(Cow::Borrowed(UI_STRINGS.settings_bool.unknown_row), |key| {
        Cow::Borrowed(key.toml_key())
    })
}

#[must_use]
pub fn overlay_bool(overlay: &UblxOverlay, scope: SettingsConfigScope, idx: usize) -> bool {
    bool_key(scope, idx).is_some_and(|key| {
        key.overlay_opt(overlay)
            .unwrap_or(key.default_when_missing())
    })
}

pub fn write_bool(overlay: &mut UblxOverlay, scope: SettingsConfigScope, idx: usize, v: bool) {
    if let Some(key) = bool_key(scope, idx) {
        key.write(overlay, v);
    }
}
