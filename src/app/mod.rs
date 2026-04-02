//! TUI app loop and view building. The loop runs in [`main_loop`]; setup/teardown live in [`crate::handlers::core::run_tui_session`].

mod delta;
mod params;
mod runtime;
mod snapshot;
pub mod tokio_rt;
mod user_selected;
mod view_data;

pub use params::*;
pub use runtime::main_loop;
pub use snapshot::load_snapshot_for_tui;
