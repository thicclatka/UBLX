//! Parameters for the TUI app loop.

use std::path::Path;
use std::sync::mpsc;

use crate::utils::notifications;

/// Parameters for the TUI event loop. Passed from [crate::handlers::core::run_ublx] into [super::main_app_loop].
pub struct RunUblxParams<'a> {
    pub db_path: &'a Path,
    pub dir_to_ublx: &'a Path,
    pub snapshot_done_rx: Option<mpsc::Receiver<(usize, usize, usize)>>,
    pub snapshot_done_tx: Option<mpsc::Sender<(usize, usize, usize)>>,
    pub bumper: Option<&'a notifications::BumperBuffer>,
    pub dev: bool,
    pub theme: Option<String>,
    pub transparent: bool,
}
