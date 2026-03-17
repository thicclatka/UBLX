//! Index DB and related files under the dir_to_ublx_abs, all keyed by package name (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).

use std::fs;
use std::path::Path;
use std::sync::mpsc::Receiver;

use rusqlite::{Connection, OptionalExtension};

use super::consts::{DeltaType, UblxDbCategory, UblxDbSchema, UblxDbStatements};
use super::utils as db_ops_utils;
use crate::config::{UblxPaths, UblxSettings};
use crate::handlers::nefax_ops::{NefaxDiff, NefaxResult};
use crate::handlers::zahir_ops::{ZahirOutput, ZahirResult, get_zahir_output_by_path};
use crate::utils::canonicalize_dir_to_ublx;

/// One row from snapshot for TUI list: (path, category, size_bytes). zahir_json is loaded on demand for the selected row.
pub type SnapshotTuiRow = (String, String, u64);

/// Write nefax + zahir outputs to the snapshot: build DB at `dir_to_ublx_abs/.ublx_tmp` (with schema), insert all rows, write settings and delta_log, then rename to `dir_to_ublx_abs/.ublx`. Uses `prior_zahir_json` for paths not in this run's zahir result (e.g. when zahir was skipped due to unchanged mtime). When `zahir_result` is None (no paths to zahir), all paths use prior.
pub fn write_snapshot_to_db(
    dir_to_ublx: &Path,
    nefax: &NefaxResult,
    zahir_result: Option<&ZahirResult>,
    diff: &NefaxDiff,
    settings: &UblxSettings,
    prior_zahir_json: &std::collections::HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    let ublx_paths = UblxPaths::new(dir_to_ublx);
    let tmp_path = ublx_paths.tmp();
    let db_path = ublx_paths.db();

    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let zahir_output_by_path = zahir_result
        .map(|z| get_zahir_output_by_path(z, Some(&dir_to_ublx_abs)))
        .unwrap_or_default();

    let conn = Connection::open(&tmp_path)?;
    conn.execute_batch(&UblxDbSchema::create_ublx_db_sql())?;
    let mut stmt = conn.prepare(UblxDbStatements::INSERT_SNAPSHOT)?;

    db_ops_utils::insert_results_into_snapshot(
        &mut stmt,
        nefax,
        dir_to_ublx,
        Some(&ublx_paths),
        &zahir_output_by_path,
        prior_zahir_json,
    )?;
    drop(stmt);

    write_settings(&conn, settings)?;
    copy_previous_delta_log(&conn, &db_path)?;
    copy_previous_lens_tables(&conn, &db_path)?;
    write_delta_log(&conn, nefax, diff)?;
    drop(conn);

    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    fs::rename(&tmp_path, &db_path)?;
    Ok(())
}

/// Write snapshot by inserting all nefax rows first (zahir from prior or empty), then consuming `output_rx`
/// and updating category + zahir_json per row. Call when zahir streams output via `OutputSink::Channel`.
pub fn write_snapshot_to_db_streaming(
    dir_to_ublx: &Path,
    nefax: &NefaxResult,
    diff: &NefaxDiff,
    settings: &UblxSettings,
    output_rx: Receiver<(String, ZahirOutput)>,
    prior_zahir_json: &std::collections::HashMap<String, String>,
) -> Result<(), anyhow::Error> {
    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let ublx_paths = UblxPaths::new(dir_to_ublx);
    let tmp_path = ublx_paths.tmp();
    let db_path = ublx_paths.db();

    let conn = Connection::open(&tmp_path)?;
    conn.execute_batch(&UblxDbSchema::create_ublx_db_sql())?;

    // Insert all nefax rows (zahir from prior when mtime unchanged, else "" until streamed).
    let mut insert_stmt = conn.prepare(UblxDbStatements::INSERT_SNAPSHOT)?;
    db_ops_utils::insert_nefax_only_into_snapshot(
        &mut insert_stmt,
        nefax,
        dir_to_ublx,
        Some(&ublx_paths),
        prior_zahir_json,
    )?;
    drop(insert_stmt);

    // Apply streamed zahir results: UPDATE category and zahir_json per path
    let mut update_stmt = conn.prepare(UblxDbStatements::UPDATE_SNAPSHOT_ZAHIR)?;
    while let Ok((path_abs, output)) = output_rx.recv() {
        let path_str = match Path::new(&path_abs).strip_prefix(&dir_to_ublx_abs) {
            Ok(rel) => rel.to_string_lossy().into_owned(),
            Err(_) => continue,
        };
        let full_path = dir_to_ublx_abs.join(&path_str);
        let category = UblxDbCategory::get_category_for_path(
            &full_path,
            Some(&ublx_paths),
            output.file_type.as_deref(),
        );
        let zahir_json = serde_json::to_string(&output).unwrap_or_default();
        let _ = update_stmt.execute(rusqlite::params![category, zahir_json, path_str]);
    }
    drop(update_stmt);

    write_settings(&conn, settings)?;
    copy_previous_delta_log(&conn, &db_path)?;
    copy_previous_lens_tables(&conn, &db_path)?;
    write_delta_log(&conn, nefax, diff)?;
    drop(conn);

    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    fs::rename(&tmp_path, &db_path)?;
    Ok(())
}

fn write_settings(conn: &Connection, s: &UblxSettings) -> Result<(), anyhow::Error> {
    conn.execute(
        UblxDbStatements::INSERT_SETTINGS,
        rusqlite::params![
            s.num_threads as i64,
            s.drive_type,
            if s.parallel_walk { 1i64 } else { 0i64 },
            s.config_source.as_deref(),
        ],
    )?;
    Ok(())
}

/// Copy all rows from the existing .ublx delta_log into the open tmp DB, so history persists
/// across the replace. No-op if db_path does not exist or has no delta_log table.
fn copy_previous_delta_log(conn: &Connection, db_path: &Path) -> Result<(), anyhow::Error> {
    if !db_path.exists() {
        return Ok(());
    }
    let path_abs = fs::canonicalize(db_path)?;
    let path_str = path_abs
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("db path not UTF-8"))?
        .replace('\'', "''");
    conn.execute(UblxDbStatements::ATTACH_OLD_DB, rusqlite::params![path_str])?;
    let copied = match conn.query_row(UblxDbStatements::SELECT_COUNT_DELTA_LOG_ROWS, [], |row| {
        row.get::<_, i32>(0)
    }) {
        Ok(1) => {
            let n = conn.execute(UblxDbStatements::COPY_PREVIOUS_DELTA_LOG, [])?;
            n as i32
        }
        _ => 0,
    };
    conn.execute(UblxDbStatements::DETACH_OLD_DB, [])?;
    if copied > 0 {
        log::debug!("copied {} previous delta_log rows into tmp", copied);
    }
    Ok(())
}

/// Copy path, lens, and lens_path from the existing .ublx into the open tmp DB so lenses persist
/// across snapshot replace. No-op if db_path does not exist or has no lens table.
fn copy_previous_lens_tables(conn: &Connection, db_path: &Path) -> Result<(), anyhow::Error> {
    if !db_path.exists() {
        return Ok(());
    }
    let path_abs = fs::canonicalize(db_path)?;
    let path_str = path_abs
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("db path not UTF-8"))?
        .replace('\'', "''");
    conn.execute(UblxDbStatements::ATTACH_OLD_DB, rusqlite::params![path_str])?;
    let has_lens = matches!(
        conn.query_row(UblxDbStatements::SELECT_LENS_TABLE_EXISTS, [], |row| row
            .get::<_, i32>(0)),
        Ok(1)
    );
    if has_lens {
        conn.execute(UblxDbStatements::COPY_PREVIOUS_PATH, [])?;
        conn.execute(UblxDbStatements::COPY_PREVIOUS_LENS, [])?;
        let n = conn.execute(UblxDbStatements::COPY_PREVIOUS_LENS_PATH, [])?;
        log::debug!(
            "copied previous lens tables into tmp ({} lens_path rows)",
            n
        );
    }
    conn.execute(UblxDbStatements::DETACH_OLD_DB, [])?;
    Ok(())
}

fn write_delta_log(
    conn: &Connection,
    nefax: &NefaxResult,
    diff: &NefaxDiff,
) -> Result<(), anyhow::Error> {
    let mut stmt = conn.prepare(UblxDbStatements::INSERT_DELTA_LOG)?;
    let created_ns = db_ops_utils::get_created_ns();

    for delta_type in DeltaType::iter() {
        db_ops_utils::insert_results_into_delta_log_by_type(
            &mut stmt, nefax, diff, delta_type, created_ns,
        )?;
    }

    Ok(())
}

/// Ensure settings table has config_source column (migration for existing DBs).
fn ensure_settings_config_source(conn: &Connection) {
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN config_source TEXT", []);
}

/// Load cached settings from the ublx DB. Returns `None` if the settings table is empty (e.g. DB created before settings existed).
pub fn load_settings_from_db(db_path: &Path) -> Result<Option<UblxSettings>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    ensure_settings_config_source(&conn);
    let settings_query = UblxDbStatements::create_query_for_settings_from_db();
    conn.query_row(&settings_query, [], |row| {
        Ok(UblxSettings {
            num_threads: row.get::<_, i64>(0)? as usize,
            drive_type: row.get(1)?,
            parallel_walk: row.get::<_, i64>(2)? != 0,
            config_source: row.get::<_, Option<String>>(3).ok().flatten(),
        })
    })
    .optional()
    .map_err(Into::into)
}

/// Load (path, zahir_json) from the snapshot table for paths that have non-empty zahir_json. Use when reusing prior zahir for unchanged mtime. Empty if DB missing or table empty.
pub fn load_snapshot_zahir_json_map(
    db_path: &Path,
) -> Result<std::collections::HashMap<String, String>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(std::collections::HashMap::new());
    }
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_PATH_ZAHIR_JSON)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = std::collections::HashMap::new();
    for r in rows {
        let (path, json) = r?;
        out.insert(path, json);
    }
    Ok(out)
}

/// Load prior Nefax: if `dir_to_ublx/NEFAX_DB` (`.nefaxer`) exists, load from that; otherwise load from the ublx snapshot at `db_path`. Returns `None` when the chosen source is empty.
pub fn load_nefax_from_db(
    dir_to_ublx: &Path,
    db_path: &Path,
) -> Result<Option<NefaxResult>, anyhow::Error> {
    db_ops_utils::NefaxFromGivenDB::new(dir_to_ublx, db_path).load_nefax_from_given_db()
}

/// Load distinct categories from the snapshot table for the TUI. Returns empty vec if table missing or empty.
pub fn load_snapshot_categories(db_path: &Path) -> Result<Vec<String>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_CATEGORIES)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Distinct snapshot timestamps from delta_log (created_ns), newest first. Empty if table missing or empty.
pub fn load_delta_log_snapshot_timestamps(db_path: &Path) -> Result<Vec<i64>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_DELTA_LOG_SNAPSHOT_TIMESTAMPS)?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Rows in delta_log for a given delta_type: (created_ns, path), newest snapshot first, then path order.
pub fn load_delta_log_rows_by_type(
    db_path: &Path,
    delta_type: &str,
) -> Result<Vec<(i64, String)>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_DELTA_LOG_ROWS_BY_TYPE)?;
    let rows = stmt.query_map(rusqlite::params![delta_type], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Load snapshot rows (path, category, size) for the TUI. zahir_json is not loaded; use [load_zahir_json_for_path] for the selected row.
pub fn load_snapshot_rows_for_tui(
    db_path: &Path,
    category_filter: Option<&str>,
) -> Result<Vec<SnapshotTuiRow>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let mut out = Vec::new();
    if let Some(cat) = category_filter {
        let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_ROWS_FOR_TUI_BY_CATEGORY)?;
        let rows = stmt.query_map(rusqlite::params![cat], |row| {
            let size: i64 = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                size.max(0) as u64,
            ))
        })?;
        for r in rows {
            out.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_ROWS_FOR_TUI_ALL)?;
        let rows = stmt.query_map([], |row| {
            let size: i64 = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                size.max(0) as u64,
            ))
        })?;
        for r in rows {
            out.push(r?);
        }
    }
    Ok(out)
}

/// Load zahir_json for a single path (for right-pane content). Returns None if path not found or zahir_json is null/empty.
pub fn load_zahir_json_for_path(
    db_path: &Path,
    path: &str,
) -> Result<Option<String>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(None);
    }
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_ZAHIR_JSON_BY_PATH)?;
    let opt: Option<String> = stmt
        .query_row(rusqlite::params![path], |row| row.get::<_, String>(0))
        .optional()?;
    Ok(opt.filter(|s| !s.is_empty()))
}

/// Load mtime_ns for a single path (for viewer footer last-modified). Returns None if path not found.
pub fn load_mtime_for_path(db_path: &Path, path: &str) -> Result<Option<i64>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(None);
    }
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_MTIME_BY_PATH)?;
    stmt.query_row(rusqlite::params![path], |row| row.get::<_, i64>(0))
        .optional()
        .map_err(Into::into)
}

/// Row for duplicate detection: path, size, optional hash (32 bytes). Directories excluded by query.
pub type SnapshotPathSizeHash = (String, u64, Option<Vec<u8>>);

/// Load path, size, hash for non-directory snapshot rows (for duplicate grouping).
pub fn load_snapshot_path_size_hash(
    db_path: &Path,
) -> Result<Vec<SnapshotPathSizeHash>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_PATH_SIZE_HASH)?;
    let rows = stmt.query_map([], |row| {
        let size: i64 = row.get(1)?;
        Ok((
            row.get::<_, String>(0)?,
            size.max(0) as u64,
            row.get::<_, Option<Vec<u8>>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
