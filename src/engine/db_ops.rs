//! Index DB and related files under the root, all keyed by package name (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::config::{NEFAX_DB, UblxPaths};
use crate::handlers::nefax_ops::{NefaxPathMeta, NefaxResult};
use crate::handlers::zahir_ops::{ZahirOutput, ZahirResult};

/// Table: one row per path. Nefaxer columns + ublx category + optional zahirscan JSON.
const CREATE_SNAPSHOT: &str = "
CREATE TABLE IF NOT EXISTS snapshot (
    path TEXT PRIMARY KEY,
    mtime_ns INTEGER NOT NULL,
    size INTEGER NOT NULL,
    hash BLOB,
    category TEXT,
    zahir_json TEXT
);
";

const INSERT_SNAPSHOT: &str = "INSERT OR REPLACE INTO snapshot (path, mtime_ns, size, hash, category, zahir_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

/// Derive category for a path: prefer zahir `file_type`; then if path contains `.git` → "git"; if path component starts with `.` → "hidden"; else "directory" for dirs, "file" for files.
pub fn category_for_path(path: &Path, zahir_file_type: Option<&str>, is_dir: bool) -> String {
    let path_str = path.to_string_lossy();
    if path_str.contains(".git") {
        return "Git".to_string();
    }
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
    {
        return "Hidden".to_string();
    }
    let fallback = if is_dir { "Directory" } else { "File" };
    zahir_file_type
        .filter(|s| !s.eq_ignore_ascii_case("unknown"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

/// Write nefax + zahir outputs to the snapshot: build DB at `root/.ublx_tmp` (with schema), insert all rows, then rename to `root/.ublx`. Nefaxer already excluded paths per opts; we write everything nefax gives us. Errors from zahir (phase1_failed, phase2_failed) are not written; caller may use them later.
pub fn write_snapshot_to_db(
    root: &Path,
    nefax: &NefaxResult,
    zahir_result: &ZahirResult,
) -> Result<(), anyhow::Error> {
    let paths = UblxPaths::new(root);
    let tmp_path = paths.tmp();
    let db_path = paths.db();

    let zahir_output_by_path: HashMap<String, &ZahirOutput> = zahir_result
        .outputs
        .iter()
        .filter_map(|o| o.source.as_ref().map(|s| (s.clone(), o)))
        .collect();

    let conn = Connection::open(&tmp_path)?;
    conn.execute_batch(CREATE_SNAPSHOT)?;
    let mut stmt = conn.prepare(INSERT_SNAPSHOT)?;

    for (path, meta) in nefax {
        let full_path = root.join(path);
        let is_dir = fs::metadata(&full_path)
            .map(|m| m.is_dir())
            .unwrap_or(false);
        let path_str = path.to_string_lossy().into_owned();
        let zahir_output = zahir_output_by_path.get(&path_str);
        let category = category_for_path(
            path,
            zahir_output.and_then(|o| o.file_type.as_deref()),
            is_dir,
        );
        let zahir_json = zahir_output
            .map(|o| serde_json::to_string(o).unwrap_or_default())
            .unwrap_or_default();
        stmt.execute(rusqlite::params![
            path_str,
            meta.mtime_ns,
            meta.size as i64,
            meta.hash.as_ref().map(|h| h.as_slice()),
            category,
            zahir_json,
        ])?;
    }
    drop(stmt);
    drop(conn);
    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    fs::rename(&tmp_path, &db_path)?;
    Ok(())
}

/// Open or create the DB at root and ensure the snapshot table exists. Returns the DB path.
pub fn ensure_ublx_and_db(root: &Path) -> Result<PathBuf, anyhow::Error> {
    let paths = UblxPaths::new(root);
    let path = paths.db();
    let conn = Connection::open(&path)?;
    conn.execute_batch(CREATE_SNAPSHOT)?;
    Ok(path)
}

/// Load a Nefax map from nefaxer's `.nefaxer` DB (paths table). Returns `None` if file missing, unreadable, or empty.
pub fn load_nefax_from_nefaxer_file(
    nefax_path: &Path,
) -> Result<Option<NefaxResult>, anyhow::Error> {
    if !nefax_path.exists() {
        return Ok(None);
    }
    let conn = Connection::open(nefax_path)?;
    load_nefax_from_paths_table(&conn)
}

/// Load the ublx snapshot table into a Nefax-compatible map. Returns `None` if the table is empty.
fn load_nefax_from_ublx_snapshot(db_path: &Path) -> Result<Option<NefaxResult>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT path, mtime_ns, size, hash FROM snapshot")?;
    let rows = stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let mtime_ns: i64 = row.get(1)?;
        let size: i64 = row.get(2)?;
        let hash_blob: Option<Vec<u8>> = row.get(3)?;
        Ok((path, mtime_ns, size, hash_blob))
    })?;
    let nefax = rows_to_nefax(rows)?;
    if nefax.is_empty() {
        Ok(None)
    } else {
        Ok(Some(nefax))
    }
}

fn rows_to_nefax(
    rows: impl Iterator<Item = rusqlite::Result<(String, i64, i64, Option<Vec<u8>>)>>,
) -> Result<NefaxResult, anyhow::Error> {
    let mut nefax = NefaxResult::new();
    for row in rows {
        let (path_str, mtime_ns, size, hash_blob) = row?;
        let path = PathBuf::from(path_str);
        let size_u = size.try_into().unwrap_or(0);
        let hash = hash_blob.filter(|b| b.len() == 32).map(|b| {
            let mut a = [0u8; 32];
            a.copy_from_slice(&b);
            a
        });
        nefax.insert(
            path,
            NefaxPathMeta {
                mtime_ns,
                size: size_u,
                hash,
            },
        );
    }
    Ok(nefax)
}

/// Load from nefaxer's paths table (same column shape as snapshot).
fn load_nefax_from_paths_table(conn: &Connection) -> Result<Option<NefaxResult>, anyhow::Error> {
    let mut stmt = conn.prepare("SELECT path, mtime_ns, size, hash FROM paths")?;
    let rows = stmt.query_map([], |row| {
        let path: String = row.get(0)?;
        let mtime_ns: i64 = row.get(1)?;
        let size: i64 = row.get(2)?;
        let hash_blob: Option<Vec<u8>> = row.get(3)?;
        Ok((path, mtime_ns, size, hash_blob))
    })?;
    let nefax = rows_to_nefax(rows)?;
    if nefax.is_empty() {
        Ok(None)
    } else {
        Ok(Some(nefax))
    }
}

/// Load prior Nefax: if `root/NEFAX_DB` (`.nefaxer`) exists, load from that; otherwise load from the ublx snapshot at `db_path`. Returns `None` when the chosen source is empty.
pub fn load_nefax_from_db(
    root: &Path,
    db_path: &Path,
) -> Result<Option<NefaxResult>, anyhow::Error> {
    let nefax_path = root.join(NEFAX_DB);
    if nefax_path.exists() {
        load_nefax_from_nefaxer_file(&nefax_path)
    } else {
        load_nefax_from_ublx_snapshot(db_path)
    }
}

/// Remove the nefaxer index file (`NEFAX_DB`) under `root` if it exists. Call after the operation is complete (e.g. after writing ublx snapshot).
pub fn delete_nefaxer_files(root: &Path) -> Result<(), anyhow::Error> {
    let path = UblxPaths::new(root).nefax_db();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Remove temporary ublx files under `root` if they exist.
pub fn delete_ublx_tmp_files(root: &Path) -> Result<(), anyhow::Error> {
    UblxPaths::new(root).remove_aux_files()?;
    Ok(())
}

/// Remove the nefaxer index file (`NEFAX_DB`) under `root` if it exists. Call after the operation is complete (e.g. after writing ublx snapshot).
pub fn post_ublx_run_cleanup(root: &Path) -> Result<(), anyhow::Error> {
    delete_nefaxer_files(root)?;
    delete_ublx_tmp_files(root)?;
    Ok(())
}
