//! Key/Value table formatting for right-pane content (e.g. metadata, writing footprint JSON).
//! Reusable for future CSV/TSV or other tabular content.
//!
//! [scrollable_content] handles layout (padding, scrollbar, scroll clamping). Here we do
//! content-specific windowed drawing: ratatui's Table has no native scroll, so we slice to
//! visible rows and draw only that range at the correct y.

pub mod consts;
pub mod csv;
pub mod draw;
pub mod format;
pub mod ratatui_table;
pub mod schema;
pub mod sections;
pub mod walk;
pub mod xlsx;

pub use draw::*;
pub use sections::*;
