//! Handler logic for small, named features (search, ublx-settings, theme-selector, dupe-finder, lens).

pub mod enhancer;
pub mod exporter;
pub mod file_ops;
mod finders;
pub mod first_run;
pub mod lenses;
pub mod opener;
pub mod settings;
pub mod theme_selector;
pub mod ublx_switch;

pub use finders::*;
