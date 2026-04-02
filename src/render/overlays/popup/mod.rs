//! Context and list popups (`menus`) and modal dialogs (`dialogs`).
//! Startup first-run overlays live in `dialogs` (`render_startup_*`).

mod dialogs;
mod menus;
mod utils;

pub use dialogs::*;
pub use menus::*;
pub use utils::{POPUP_MENU, PopupMenuConfig};
