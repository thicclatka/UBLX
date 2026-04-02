//! Panel layout, blocks, lists, and right-pane rendering.

mod layout;
mod middle;
mod right;
mod user_selected_mode;

pub mod delta_mode;
pub mod settings_mode;
pub mod snapshot_mode;

pub use layout::*;
pub use middle::*;
pub use right::*;
pub use user_selected_mode::{draw_duplicates_panes, draw_lenses_panes};
