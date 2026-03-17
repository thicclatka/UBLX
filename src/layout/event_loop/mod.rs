//! TUI app loop and view building. The loop runs in [main_app_loop]; setup/teardown live in [crate::handlers::core::run_ublx].

mod app_loop;
mod delta;
mod duplicates;
mod lenses;
mod params;
mod snapshot;
mod view_data;

pub use app_loop::main_app_loop;
pub use params::*;
pub use snapshot::load_snapshot_for_tui;
