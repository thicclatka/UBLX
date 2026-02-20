//! Snapshot DB path resolution for reading. Use when the TUI should show live data
//! from a running snapshot (e.g. read from `.ublx_tmp` until the pipeline renames to `.ublx`).

use std::path::{Path, PathBuf};

use crate::config::UblxPaths;

fn ublx_paths_from_db_path(db_path: &Path) -> Option<UblxPaths> {
    db_path.parent().map(UblxPaths::new)
}

/// Which file to prefer when both exist: final `.ublx` or in-progress `.ublx_tmp`.
#[derive(Clone, Copy)]
pub enum SnapshotReaderPreference {
    /// Prefer `.ublx`; use `.ublx_tmp` only when `.ublx` is missing (e.g. first run).
    PreferUblx,
    /// Prefer `.ublx_tmp` so the TUI shows live data while a pipeline is running.
    PreferTmp,
}

/// Path to use for reading the snapshot. Returns `None` when neither `.ublx` nor `.ublx_tmp` exists.
pub fn snapshot_reader_path_with(
    db_path: &Path,
    preference: SnapshotReaderPreference,
) -> Option<PathBuf> {
    let paths = ublx_paths_from_db_path(db_path)?;
    let db = paths.db();
    let tmp = paths.tmp();
    let (first, second) = match preference {
        SnapshotReaderPreference::PreferUblx => (db, tmp),
        SnapshotReaderPreference::PreferTmp => (tmp, db),
    };
    if first.exists() {
        Some(first)
    } else if second.exists() {
        Some(second)
    } else {
        None
    }
}
