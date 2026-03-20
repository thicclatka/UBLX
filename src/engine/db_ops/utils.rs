use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use log::warn;
use rayon::prelude::*;
use rusqlite::{Connection, Statement};

use super::consts::{DeltaType, UblxDbCategory, UblxDbSchema, UblxDbStatements};
use crate::config::{NEFAX_DB, PARALLEL, UblxPaths};
use crate::handlers::{
    nefax_ops::{NefaxDiff, NefaxPathMeta, NefaxResult},
    zahir_ops::{ZahirOutput, zahir_output_to_json},
};

/// Get the full path and the path string for a given path.
#[must_use]
pub fn get_full_path_and_path_str(dir_to_ublx: &Path, path_ref: &Path) -> (PathBuf, String) {
    let full_path = dir_to_ublx.join(path_ref);
    let path_str = path_ref.to_string_lossy().into_owned();
    (full_path, path_str)
}

/// Open or create the DB at `dir_to_ublx` and ensure all ublx tables exist. Returns the DB path.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` I/O or schema initialization errors.
pub fn ensure_ublx_and_db(dir_to_ublx: &Path) -> Result<PathBuf, anyhow::Error> {
    let paths = UblxPaths::new(dir_to_ublx);
    let path = paths.db();
    let conn = Connection::open(&path)?;
    conn.execute_batch(&UblxDbSchema::create_ublx_db_sql())?;
    Ok(path)
}

/// Get `file_type` string from prior zahir JSON so we can preserve category when current run didn't re-run zahir.
fn file_type_from_zahir_json(json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| {
            v.get("file_type")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
}

#[must_use]
pub fn prepare_results_for_snapshot_insertion(
    dir_to_ublx: &Path,
    path_ref: &Path,
    ublx_paths: Option<&UblxPaths>,
    zahir_output_by_path: &HashMap<String, &ZahirOutput>,
    prior_zahir_json: &std::collections::HashMap<String, String>,
) -> (String, String, String) {
    let (full_path, path_str) = get_full_path_and_path_str(dir_to_ublx, path_ref);
    let zahir_output = zahir_output_by_path.get(&path_str);
    let prior_ft = prior_zahir_json
        .get(&path_str)
        .and_then(|j| file_type_from_zahir_json(j));
    let zahir_file_type = zahir_output
        .and_then(|o| o.file_type.as_deref())
        .or(prior_ft.as_deref());
    let category = UblxDbCategory::get_category_for_path(&full_path, ublx_paths, zahir_file_type);
    let zahir_json = zahir_output.map_or_else(
        || prior_zahir_json.get(&path_str).cloned().unwrap_or_default(),
        |o| zahir_output_to_json(Some(o)),
    );
    (path_str, category, zahir_json)
}

/// Prepared row for snapshot insert: (`path_str`, category, `zahir_json`, `mtime_ns`, size, hash).
/// Hash is owned so the vec can be built in parallel (Send).
type SnapshotInsertRow = (String, String, String, i64, i64, Option<Vec<u8>>);

/// Insert all nefax rows into the prepared snapshot `INSERT` statement, then optional global config row.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` execute errors or filesystem metadata errors when inserting global config.
pub fn insert_results_into_snapshot(
    stmt: &mut Statement,
    nefax: &NefaxResult,
    dir_to_ublx: &Path,
    ublx_paths: Option<&UblxPaths>,
    zahir_output_by_path: &HashMap<String, &ZahirOutput>,
    prior_zahir_json: &std::collections::HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    if nefax.len() >= PARALLEL.snapshot_insert_prep {
        let entries: Vec<_> = nefax.iter().collect();
        let prepared: Vec<SnapshotInsertRow> = entries
            .par_iter()
            .map(|(path, meta)| {
                let (path_str, category, zahir_json) = prepare_results_for_snapshot_insertion(
                    dir_to_ublx,
                    path.as_path(),
                    ublx_paths,
                    zahir_output_by_path,
                    prior_zahir_json,
                );
                let hash = meta.hash.as_ref().map(|h| h.as_slice().to_vec());
                (
                    path_str,
                    category,
                    zahir_json,
                    meta.mtime_ns,
                    meta.size.cast_signed(),
                    hash,
                )
            })
            .collect();
        for (path_str, category, zahir_json, mtime_ns, size, hash) in prepared {
            stmt.execute(rusqlite::params![
                path_str,
                mtime_ns,
                size,
                hash.as_deref(),
                category,
                zahir_json,
            ])?;
        }
    } else {
        for (path, meta) in nefax {
            let (path_str, category, zahir_json) = prepare_results_for_snapshot_insertion(
                dir_to_ublx,
                path,
                ublx_paths,
                zahir_output_by_path,
                prior_zahir_json,
            );
            stmt.execute(rusqlite::params![
                path_str,
                meta.mtime_ns,
                meta.size.cast_signed(),
                meta.hash.as_ref().map(<[u8; 32]>::as_slice),
                category,
                zahir_json,
            ])?;
        }
    }
    insert_global_config_row_if_exists(stmt, ublx_paths)?;
    Ok(())
}

/// If global config file exists, insert a row under UBLX Settings with path = absolute path to that config.
fn insert_global_config_row_if_exists(
    stmt: &mut Statement,
    ublx_paths: Option<&UblxPaths>,
) -> Result<(), anyhow::Error> {
    let Some(paths) = ublx_paths else {
        return Ok(());
    };
    let Some(global_path) = paths.global_config() else {
        return Ok(());
    };
    if !global_path.exists() {
        return Ok(());
    }
    let (mtime_ns, size) = fs::metadata(&global_path)
        .map(|m| {
            let mtime_ns = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
            let size = m.len().cast_signed();
            (mtime_ns, size)
        })
        .unwrap_or((0, 0));
    let path_str = global_path.to_string_lossy().into_owned();
    stmt.execute(rusqlite::params![
        path_str,
        mtime_ns,
        size,
        None::<&[u8]>,
        UblxDbCategory::UblxSettings.as_str(),
        "",
    ])?;
    Ok(())
}

/// Insert all nefax rows (category from prior zahir when available, else path fallback; `zahir_json` from prior). For streaming: zahir updates applied later for paths that were sent.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` execute errors or filesystem metadata errors when inserting global config.
pub fn insert_nefax_only_into_snapshot(
    stmt: &mut Statement,
    nefax: &NefaxResult,
    dir_to_ublx: &Path,
    ublx_paths: Option<&UblxPaths>,
    prior_zahir_json: &std::collections::HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    for (path, meta) in nefax {
        let (full_path, path_str) = get_full_path_and_path_str(dir_to_ublx, path);
        let prior_ft = prior_zahir_json
            .get(&path_str)
            .and_then(|j| file_type_from_zahir_json(j));
        let category =
            UblxDbCategory::get_category_for_path(&full_path, ublx_paths, prior_ft.as_deref());
        let zahir_json = prior_zahir_json.get(&path_str).cloned().unwrap_or_default();
        stmt.execute(rusqlite::params![
            path_str,
            meta.mtime_ns,
            meta.size.cast_signed(),
            meta.hash.as_ref().map(<[u8; 32]>::as_slice),
            category,
            zahir_json,
        ])?;
    }
    insert_global_config_row_if_exists(stmt, ublx_paths)?;
    Ok(())
}

#[must_use]
pub fn get_created_ns() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

/// Insert `delta_log` rows for one delta type (added/modified/removed).
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` execute errors.
pub fn insert_results_into_delta_log_by_type(
    stmt: &mut Statement,
    nefax: &NefaxResult,
    diff: &NefaxDiff,
    delta_type: DeltaType,
    created_ns: i64,
) -> Result<(), anyhow::Error> {
    let diff_to_use = match delta_type {
        DeltaType::Added => &diff.added,
        DeltaType::Mod => &diff.modified,
        DeltaType::Removed => &diff.removed,
    };
    for path in diff_to_use {
        let path_str = path.to_string_lossy().into_owned();
        let (mtime_ns, size, hash) = match delta_type {
            DeltaType::Added | DeltaType::Mod => {
                // Paths in diff.added/diff.modified should match the current nefax run; if not, skip.
                let Some(meta) = nefax.get(path) else {
                    warn!(
                        "nefax missing metadata for {} path {}; skipping delta_log row",
                        delta_type.as_str(),
                        path_str
                    );
                    continue;
                };
                (
                    Some(meta.mtime_ns),
                    Some(meta.size.cast_signed()),
                    meta.hash.as_ref().map(<[u8; 32]>::as_slice),
                )
            }
            DeltaType::Removed => (None::<i64>, None::<i64>, None::<&[u8]>),
        };
        stmt.execute(rusqlite::params![
            created_ns,
            path_str,
            mtime_ns,
            size,
            hash,
            delta_type.as_str(),
        ])?;
    }
    Ok(())
}

pub struct NefaxFromGivenDB {
    db_path_to_use: PathBuf,
    table_name: String,
}

impl NefaxFromGivenDB {
    #[must_use]
    pub fn new(dir_to_ublx: &Path, db_path: &Path) -> Self {
        let (db_path_to_use, table_name) =
            Self::determine_db_path_and_table_name(dir_to_ublx, db_path);
        Self {
            db_path_to_use,
            table_name: table_name.to_string(),
        }
    }

    fn determine_db_path_and_table_name(
        dir_to_ublx: &Path,
        db_path: &Path,
    ) -> (PathBuf, &'static str) {
        let nefax_path = dir_to_ublx.join(NEFAX_DB);
        if nefax_path.exists() {
            (nefax_path, "paths")
        } else {
            (db_path.to_path_buf(), "snapshot")
        }
    }

    /// Load the ublx snapshot table into a Nefax-compatible map. Returns `None` if the table is empty.
    ///
    /// # Errors
    ///
    /// Returns [`anyhow::Error`] on `SQLite` open/query errors.
    pub fn load_nefax_from_given_db(self) -> Result<Option<NefaxResult>, anyhow::Error> {
        let conn = Connection::open(&self.db_path_to_use)?;
        let query = UblxDbStatements::create_query_for_nefax_from_db(&self.table_name);

        let mut stmt = conn.prepare(&query)?;

        let rows = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let mtime_ns: i64 = row.get(1)?;
            let size: i64 = row.get(2)?;
            let hash_blob: Option<Vec<u8>> = row.get(3)?;
            Ok((path, mtime_ns, size, hash_blob))
        })?;
        let nefax = Self::rows_to_nefax(rows)?;
        if nefax.is_empty() {
            Ok(None)
        } else {
            Ok(Some(nefax))
        }
    }

    /// Convert a list of rows to a Nefax-compatible map.
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
}

pub struct UblxCleanup {
    dir_to_ublx_abs: PathBuf,
}

impl UblxCleanup {
    #[must_use]
    pub fn new(dir_to_ublx: &Path) -> Self {
        Self {
            dir_to_ublx_abs: dir_to_ublx.to_path_buf(),
        }
    }

    /// Remove the nefaxer index file (`NEFAX_DB`) under `dir_to_ublx` if it exists. Call after the operation is complete (e.g. after writing ublx snapshot).
    ///
    /// # Errors
    ///
    /// Returns [`anyhow::Error`] when removing the file fails.
    pub fn delete_nefaxer_files(dir_to_ublx: &Path) -> Result<(), anyhow::Error> {
        let path = UblxPaths::new(dir_to_ublx).nefax_db();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Remove temporary ublx files under `dir_to_ublx` if they exist.
    ///
    /// # Errors
    ///
    /// Returns [`anyhow::Error`] when removing auxiliary files fails.
    pub fn delete_ublx_tmp_files(dir_to_ublx: &Path) -> Result<(), anyhow::Error> {
        UblxPaths::new(dir_to_ublx).remove_aux_files()?;
        Ok(())
    }

    /// Remove the nefaxer index file and ublx tmp files under this cleanup's `dir_to_ublx_abs`. Call after the operation is complete (e.g. after writing ublx snapshot).
    ///
    /// # Errors
    ///
    /// Returns [`anyhow::Error`] when deleting nefaxer or tmp files fails.
    pub fn post_run_cleanup(&self) -> Result<(), anyhow::Error> {
        Self::delete_nefaxer_files(&self.dir_to_ublx_abs)?;
        Self::delete_ublx_tmp_files(&self.dir_to_ublx_abs)?;
        Ok(())
    }
}
