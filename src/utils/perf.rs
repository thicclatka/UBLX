//! Timing helpers: set **`UBLX_PROFILE=1`** and `RUST_LOG=ublx_perf=debug` (or `RUST_LOG=debug`).

use std::sync::OnceLock;
use std::time::Instant;

use log::debug;

static PROFILE: OnceLock<bool> = OnceLock::new();

#[inline]
#[must_use]
pub fn profile_enabled() -> bool {
    *PROFILE.get_or_init(|| {
        std::env::var("UBLX_PROFILE").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    })
}

/// When profiling is on, logs elapsed time on drop with target `ublx_perf`.
pub struct PerfGuard {
    label: &'static str,
    start: Option<Instant>,
}

impl PerfGuard {
    #[must_use]
    pub fn new(label: &'static str) -> Self {
        let start = profile_enabled().then(Instant::now);
        Self { label, start }
    }
}

impl Drop for PerfGuard {
    fn drop(&mut self) {
        if let Some(t0) = self.start.take() {
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            debug!(target: "ublx_perf", "{}: {:.2} ms", self.label, ms);
        }
    }
}
