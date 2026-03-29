//! Parameters for the TUI app loop.

use std::path::Path;
use std::sync::mpsc;

use tokio::sync::mpsc::UnboundedSender;

use crate::config::LayoutOverlay;
use crate::engine::db_ops::DuplicateGroup;
use crate::layout::setup::RightPaneAsyncReady;
use crate::utils;

/// Dev logging (from CLI / config).
#[derive(Clone, Copy, Debug, Default)]
pub struct RunUblxDisplayOpts {
    pub dev: bool,
}

/// First-run enhance prompt and deferred “full Zahir in background” toast.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunUblxStartupFlow {
    /// First launch in this dir: show enhance-all prompt; defer first background snapshot until answered.
    pub defer_first_snapshot: bool,
    /// When set, next tick shows a toast that index-time Zahir is running in the background ([`crate::engine::orchestrator::should_force_full_zahir`]).
    pub pending_force_full_enhance_toast: bool,
}

/// Parameters for the TUI event loop. Passed from [`crate::handlers::core::run_tui_session`] into [`super::main_loop`].
pub struct RunUblxParams<'a> {
    pub db_path: &'a Path,
    pub dir_to_ublx: &'a Path,
    pub snapshot_done_rx: Option<mpsc::Receiver<(usize, usize, usize)>>,
    pub snapshot_done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    pub bumper: Option<&'a utils::BumperBuffer>,
    pub display: RunUblxDisplayOpts,
    pub theme: Option<String>,
    /// Left/middle/right pane percentages (0–100). Hot-reloadable from config [layout].
    pub layout: LayoutOverlay,
    /// Duplicate groups (lazy-loaded when user switches to Duplicates tab). Empty until load completes.
    pub duplicate_groups: Vec<DuplicateGroup>,
    /// When some, a background thread is loading duplicate groups; main loop receives via `try_recv`.
    pub duplicate_groups_rx: Option<mpsc::Receiver<Vec<DuplicateGroup>>>,
    /// Lens names for the Lenses tab (loaded at startup). When non-empty, Lenses tab is shown.
    pub lens_names: Vec<String>,
    /// When some, a file watcher sends () on global/local config save; main loop triggers hot reload.
    pub config_reload_rx: Option<mpsc::Receiver<()>>,
    pub startup: RunUblxStartupFlow,
    /// When set, file-row right-pane content is resolved on a background worker.
    pub right_pane_async_tx: Option<UnboundedSender<RightPaneAsyncReady>>,
}
