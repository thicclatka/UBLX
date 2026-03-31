//! Main tab bar: digit keys, labels, and tab-string helpers.

use crate::engine::db_ops::DuplicateGroupingMode;

/// Digit keys (1–9) for main-mode tabs. Single source for keymap, tab labels, mouse hit-testing, and help.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UblxTabNumber {
    pub snapshot: u8,
    pub delta: u8,
    pub settings: u8,
    pub lenses: u8,
    pub duplicates: u8,
}

impl UblxTabNumber {
    /// Snapshot [1], Lenses [2], Delta [7], Duplicates [8], Settings [9] — matches left-to-right tab bar order.
    pub const DEFAULT: Self = Self {
        snapshot: 1,
        lenses: 2,
        delta: 7,
        duplicates: 8,
        settings: 9,
    };
}

pub const MAIN_TAB_KEYS: UblxTabNumber = UblxTabNumber::DEFAULT;

/// Tab label with hotkey digit, e.g. `Settings [9]` — use with [`MAIN_TAB_KEYS`] and [`UiStringsMainTabs`].
#[must_use]
pub fn main_tab_title(label: &str, key_digit: u8) -> String {
    format!("{label} [{key_digit}]")
}

/// Main mode tab bar: Snapshot | Delta | …
pub struct UiStringsMainTabs {
    pub snapshot: &'static str,
    pub delta: &'static str,
    pub settings: &'static str,
    pub duplicates: &'static str,
    pub lenses: &'static str,
}

impl UiStringsMainTabs {
    #[must_use]
    pub fn duplicates_with_mode(&self, mode: DuplicateGroupingMode) -> String {
        match mode {
            DuplicateGroupingMode::Hash => format!("{} (H)", self.duplicates),
            DuplicateGroupingMode::NameSize => format!("{} (N/S)", self.duplicates),
        }
    }
}
