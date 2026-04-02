//! Main app loop: one tick = side effects → toasts/snapshot → build view + right content → draw → input.
//!
//! | Module        | Role                                                                 |
//! | ------------- | -------------------------------------------------------------------- |
//! | [`tick`]      | [`run_tick`]: orchestrates one frame (applets, snapshot, draw, input). |
//! | [`view_build`] | [`build_view_and_right_content`] for Snapshot / Delta / Duplicates / Lenses. |
//! | [`frame`]     | [`DrawFrameArgs`] + terminal draw + theme name for the frame.        |

use std::io;

use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::app::RunUblxParams;
use crate::config::UblxOpts;
use crate::layout::setup;

mod frame;
mod tick;
mod view_build;

/// Runs until the user quits. Call from [`crate::handlers::core::run_tui_session`] after terminal setup.
///
/// Per tick: see [`tick::run_tick`].
///
/// # Errors
///
/// Returns [`std::io::Error`] when terminal draw, input, or snapshot I/O fails.
pub fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut setup::UblxState,
    categories: &mut Vec<String>,
    all_rows: &mut Vec<setup::TuiRow>,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) -> io::Result<()> {
    loop {
        if tick::run_tick(terminal, state, categories, all_rows, params, ublx_opts)? {
            break;
        }
    }
    Ok(())
}
