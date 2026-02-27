//! Handler logic for small, named features (ublx-settings, theme-selector, dupe-finder).
//! Layout/config/render stay elsewhere; these modules own event handling and state updates.

pub mod dupe_finder;
pub mod settings;
pub mod theme_selector;
