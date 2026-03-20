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
#[must_use]
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

/// Path the TUI should use for reading snapshot data (list rows, `zahir_json`, mtime). When `prefer_tmp` is true (e.g. snapshot still running), prefers `.ublx_tmp` so the UI shows live progress; otherwise prefers `.ublx`. Falls back to `db_path` if neither exists.
#[must_use]
pub fn snapshot_read_path_for_tui(db_path: &Path, prefer_tmp: bool) -> PathBuf {
    let preference = if prefer_tmp {
        SnapshotReaderPreference::PreferTmp
    } else {
        SnapshotReaderPreference::PreferUblx
    };
    snapshot_reader_path_with(db_path, preference).unwrap_or_else(|| db_path.to_path_buf())
}
