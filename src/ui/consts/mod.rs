//! UI string tables, glyphs, tree characters, and layout constants.

mod command_mode;
mod glyph;
mod layout;
mod strings;
mod tabs;

pub use command_mode::*;
pub use glyph::*;
pub use layout::*;
pub use strings::*;
pub use tabs::*;

use crate::engine::db_ops::DuplicateGroupingMode;
use crate::layout::setup::MainMode;

/// Main tab bar order and labels (Snapshot, optional Lenses, Delta, optional Duplicates, Settings).
/// Matches [`crate::render::core::draw_main_tabs`] segment order and mouse hit-testing.
#[must_use]
pub fn main_tab_bar_modes_and_labels(
    has_lenses: bool,
    has_duplicates: bool,
    duplicate_mode: DuplicateGroupingMode,
) -> (Vec<MainMode>, Vec<String>) {
    let k = MAIN_TAB_KEYS;
    let mut modes = vec![MainMode::Snapshot];
    let mut labels = vec![main_tab_title(UI_STRINGS.main_tabs.snapshot, k.snapshot)];
    if has_lenses {
        modes.push(MainMode::Lenses);
        labels.push(main_tab_title(UI_STRINGS.main_tabs.lenses, k.lenses));
    }
    modes.push(MainMode::Delta);
    labels.push(main_tab_title(UI_STRINGS.main_tabs.delta, k.delta));
    if has_duplicates {
        modes.push(MainMode::Duplicates);
        let duplicates_label = UI_STRINGS.main_tabs.duplicates_with_mode(duplicate_mode);
        labels.push(main_tab_title(&duplicates_label, k.duplicates));
    }
    modes.push(MainMode::Settings);
    labels.push(main_tab_title(UI_STRINGS.main_tabs.settings, k.settings));
    (modes, labels)
}

/// Help header: digits in tab-bar order (Snapshot, optional Lenses, Delta, optional Duplicates, Settings).
/// Matches [`main_tab_bar_modes_and_labels`]: only keys for tabs that are **shown** are listed.
#[must_use]
pub fn main_tab_keys_help_keys_line(has_lenses: bool, has_duplicates: bool) -> String {
    let k = MAIN_TAB_KEYS;
    let mut parts: Vec<String> = Vec::new();
    parts.push(k.snapshot.to_string());
    if has_lenses {
        parts.push(k.lenses.to_string());
    }
    parts.push(k.delta.to_string());
    if has_duplicates {
        parts.push(k.duplicates.to_string());
    }
    parts.push(k.settings.to_string());
    parts.join(" | ")
}
