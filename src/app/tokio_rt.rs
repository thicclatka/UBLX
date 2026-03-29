//! Shared Tokio runtime for background work (e.g. right-pane file + DB resolve) while the TUI thread stays on crossterm.

use std::sync::OnceLock;

use tokio::runtime::Runtime;

use crate::config::TOKIO_RUNTIME_WORKERS;

static RT: OnceLock<Runtime> = OnceLock::new();

#[must_use]
/// Start Tokio runtime if not already started.
///
/// # Panics
///
/// Runtime fails to start.
pub fn runtime() -> &'static Runtime {
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(TOKIO_RUNTIME_WORKERS)
            .thread_name("ublx-tokio")
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}
