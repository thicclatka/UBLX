//! Shared TUI styles and layout constants for the 3-panel layout.
//! Colors come from the current theme ([crate::layout::themes::current]); set at frame start via [crate::layout::themes::set_current].

mod core;
mod nodes;
mod table;

pub use core::*;
pub use nodes::*;
pub use table::*;
