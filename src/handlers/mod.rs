//! TUI event handlers: session lifecycle ([`core`]), snapshot/background pipeline, mode transitions, and
//! the right-hand preview stack ([`viewing`]).

mod core;
mod snapshot_pipeline;
pub mod state_transitions;
pub mod viewing;

pub use core::*;
pub use snapshot_pipeline::*;
