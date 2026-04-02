//! Shared TUI styles and layout constants for the 3-panel layout.
//!
//! **Ownership vs [`crate::themes`]:** this module applies styles and layout rules (geometry, focus, bold/reverse, borders). It does *not* own palette data. Colors come from the active palette via [`crate::themes::current`]; the frame’s theme name is set at draw start with [`crate::themes::set_current`]. Prefer importing `crate::themes` here rather than defining or duplicating palette fields.
//!
//! Dependency direction: `layout::style` → `themes` only; `themes` must not depend on `layout` (avoids cycles and keeps “what it looks like” separate from “where it goes”).

mod core;
mod nodes;
mod table;

pub use core::*;
pub use nodes::*;
pub use table::*;
