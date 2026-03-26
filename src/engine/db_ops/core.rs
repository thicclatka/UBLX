//! Index DB and related files under the `dir_to_ublx_abs`, all keyed by package name (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).

use std::fs;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use log::debug;
use rusqlite::{Connection, OptionalExtension};

use super::consts::{DeltaType, UblxDbSchema, UblxDbStatements};
use super::utils::{self as db_ops_utils, SnapshotPriorContext};

use crate::config::{UblxPaths, UblxSettings};
use crate::handlers::{
    nefax_ops::{NefaxDiff, NefaxResult},
    zahir_ops::{
        ZahirOutput, ZahirResult, get_zahir_output_by_path, zahir_output_to_json_for_path,
    },
};
use crate::utils::{canonicalize_dir_to_ublx, get_created_ns, normalize_snapshot_rel_path_str};

/// How long TUI snapshot reads wait on `SQLITE_BUSY` before failing (keeps the event loop responsive while tmp is written).
pub const SNAPSHOT_TUI_READ_BUSY_MS: u64 = 2;

/// Open the DB for interactive reads with a short busy timeout (poll / right pane while snapshot runs).
///
/// # Errors
///
/// Returns any `SQLite` open error for `path` and any failure applying the busy timeout pragma.
pub fn open_for_snapshot_tui_read(path: &Path) -> Result<Connection, rusqlite::Error> {
    let c = Connection::open(path)?;
    c.busy_timeout(Duration::from_millis(SNAPSHOT_TUI_READ_BUSY_MS))?;
    Ok(c)
}

/// WAL on `.ublx_tmp` so readers (TUI poll) can overlap snapshot writes; finalized before rename.
fn apply_wal_for_snapshot_tmp(conn: &Connection) -> Result<(), anyhow::Error> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}

/// Merge WAL into the main DB file and switch to DELETE journal before atomic rename to `.ublx`.
fn finalize_snapshot_tmp_before_rename(conn: &Connection) -> Result<(), anyhow::Error> {
    conn.execute_batch(
        "PRAGMA wal_checkpoint(TRUNCATE);
         PRAGMA journal_mode = DELETE;",
    )?;
    Ok(())
}

fn strip_tmp_wal_shm_best_effort(paths: &UblxPaths) {
    for p in [paths.tmp_wal(), paths.tmp_shm()] {
        let _ = fs::remove_file(p);
    }
}

/// Run `body` in one `SQLite` transaction (`BEGIN` … `COMMIT`). Rolls back if `body` or `commit` fails.
fn in_transaction<T>(
    conn: &mut Connection,
    body: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, anyhow::Error>,
) -> Result<T, anyhow::Error> {
    let tx = conn.transaction()?;
    let out = body(&tx)?;
    tx.commit()?;
    Ok(out)
}

/// Settings tx, then `ATTACH` live `.ublx` copy of `delta_log`/lens (not inside a `main` write tx —
/// `SQLCipher` can return `database old is locked`), then `delta_log` tx.
fn write_settings_copy_previous_write_delta_log(
    conn: &mut Connection,
    live_db_path: &Path,
    settings: &UblxSettings,
    nefax: &NefaxResult,
    diff: &NefaxDiff,
) -> Result<(), anyhow::Error> {
    in_transaction(conn, |tx| {
        write_settings(tx, settings)?;
        Ok(())
    })?;
    copy_previous_aux_tables(conn, live_db_path)?;
    in_transaction(conn, |tx| {
        write_delta_log(tx, nefax, diff)?;
        Ok(())
    })?;
    Ok(())
}

/// One row from snapshot for TUI list: (path, category, `size_bytes`). `zahir_json` is loaded on demand for the selected row.
pub type SnapshotTuiRow = (String, String, u64);

/// Write nefax + zahir outputs to the snapshot: build DB at `dir_to_ublx_abs/.ublx_tmp` (with schema), insert all rows, write settings and `delta_log`, then rename to `dir_to_ublx_abs/.ublx`. Uses `prior.prior_zahir_json` for paths not in this run's zahir result (e.g. when zahir was skipped due to unchanged mtime). Uses `prior.prior_category` so listing categories stay stable when only `zahir_json` changes. When `zahir_result` is None (no paths to zahir), all paths use prior.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` I/O, query/prepare errors, or filesystem errors when replacing the DB file.
pub fn write_snapshot_to_db(
    dir_to_ublx: &Path,
    nefax: &NefaxResult,
    zahir_result: Option<&ZahirResult>,
    diff: &NefaxDiff,
    settings: &UblxSettings,
    prior: &SnapshotPriorContext<'_>,
) -> Result<(), anyhow::Error> {
    let ublx_paths = UblxPaths::new(dir_to_ublx);
    let tmp_path = ublx_paths.tmp();
    let db_path = ublx_paths.db();

    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let zahir_output_by_path = zahir_result
        .map(|z| get_zahir_output_by_path(z, Some(&dir_to_ublx_abs)))
        .unwrap_or_default();

    debug!(
        "snapshot DB (sequential): tmp={} final={} nefax_paths={} zahir_paths_in_result={}",
        tmp_path.display(),
        db_path.display(),
        nefax.len(),
        zahir_output_by_path.len()
    );

    let mut conn = Connection::open(&tmp_path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch(&UblxDbSchema::create_ublx_db_sql())?;
    apply_wal_for_snapshot_tmp(&conn)?;

    in_transaction(&mut conn, |tx| {
        let mut stmt = tx.prepare(UblxDbStatements::INSERT_SNAPSHOT)?;
        db_ops_utils::insert_results_into_snapshot(
            &mut stmt,
            nefax,
            dir_to_ublx,
            Some(&ublx_paths),
            &zahir_output_by_path,
            prior,
        )?;
        Ok(())
    })?;

    debug!(
        "snapshot DB (sequential): inserted snapshot rows, writing settings/delta_log then replace"
    );

    write_settings_copy_previous_write_delta_log(&mut conn, &db_path, settings, nefax, diff)?;
    finalize_snapshot_tmp_before_rename(&conn)?;
    drop(conn);
    strip_tmp_wal_shm_best_effort(&ublx_paths);

    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    fs::rename(&tmp_path, &db_path)?;
    debug!("snapshot DB (sequential): committed {}", db_path.display());
    Ok(())
}

/// Write snapshot by inserting all nefax rows first (zahir from prior or empty), then consuming `output_rx`
/// and updating `zahir_json` per row (category unchanged from insert). Call when zahir streams output via `OutputSink::Channel`.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` I/O, query/prepare errors, or filesystem errors when replacing the DB file.
pub fn write_snapshot_to_db_streaming(
    dir_to_ublx: &Path,
    nefax: &NefaxResult,
    diff: &NefaxDiff,
    settings: &UblxSettings,
    output_rx: &Receiver<(String, ZahirOutput)>,
    prior: &SnapshotPriorContext<'_>,
) -> Result<(), anyhow::Error> {
    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let ublx_paths = UblxPaths::new(dir_to_ublx);
    let tmp_path = ublx_paths.tmp();
    let db_path = ublx_paths.db();

    debug!(
        "snapshot DB (streaming): tmp={} final={} nefax_paths={}",
        tmp_path.display(),
        db_path.display(),
        nefax.len()
    );

    let mut conn = Connection::open(&tmp_path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch(&UblxDbSchema::create_ublx_db_sql())?;
    apply_wal_for_snapshot_tmp(&conn)?;

    // Insert all nefax rows (zahir from prior when mtime unchanged, else "" until streamed).
    in_transaction(&mut conn, |tx| {
        let mut insert_stmt = tx.prepare(UblxDbStatements::INSERT_SNAPSHOT)?;
        db_ops_utils::insert_nefax_only_into_snapshot(
            &mut insert_stmt,
            nefax,
            dir_to_ublx,
            Some(&ublx_paths),
            prior,
        )?;
        Ok(())
    })?;

    debug!(
        "snapshot DB (streaming): inserted {} snapshot rows; waiting for zahir stream (0 updates is normal when index-time Zahir is off)",
        nefax.len()
    );

    // Apply streamed zahir results: `zahir_json` only (category unchanged from insert).
    let zahir_updates = in_transaction(&mut conn, |tx| {
        let mut n = 0u64;
        let mut update_stmt = tx.prepare(UblxDbStatements::UPDATE_SNAPSHOT_ZAHIR_JSON_ONLY)?;
        while let Ok((path_abs, output)) = output_rx.recv() {
            let path_str = match Path::new(&path_abs).strip_prefix(&dir_to_ublx_abs) {
                Ok(rel) => rel.to_string_lossy().into_owned(),
                Err(_) => continue,
            };
            let full_path = dir_to_ublx_abs.join(Path::new(&path_str));
            let zahir_json = zahir_output_to_json_for_path(Some(&output), &full_path, &path_str);
            let _ = update_stmt.execute(rusqlite::params![zahir_json, path_str]);
            n += 1;
            db_ops_utils::debug_snapshot_write_progress("streamed zahir updates", n, None);
        }
        Ok(n)
    })?;

    debug!(
        "snapshot DB (streaming): zahir stream finished ({zahir_updates} updates); writing settings/delta_log then replace"
    );

    write_settings_copy_previous_write_delta_log(&mut conn, &db_path, settings, nefax, diff)?;
    finalize_snapshot_tmp_before_rename(&conn)?;
    drop(conn);
    strip_tmp_wal_shm_best_effort(&ublx_paths);

    if db_path.exists() {
        fs::remove_file(&db_path)?;
    }
    fs::rename(&tmp_path, &db_path)?;
    debug!("snapshot DB (streaming): committed {}", db_path.display());
    Ok(())
}

fn write_settings(conn: &Connection, s: &UblxSettings) -> Result<(), anyhow::Error> {
    conn.execute(
        UblxDbStatements::INSERT_SETTINGS,
        rusqlite::params![
            i64::try_from(s.num_threads).unwrap_or(i64::MAX),
            s.drive_type,
            i64::from(s.parallel_walk),
            s.config_source.as_deref(),
        ],
    )?;
    Ok(())
}

fn path_str_for_attach(path: &Path) -> Result<String, anyhow::Error> {
    let path_abs = fs::canonicalize(path)?;
    Ok(path_abs
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("db path not UTF-8"))?
        .replace('\'', "''"))
}

/// Copy `delta_log` and lens tables from the live `.ublx` into `conn` (main = tmp DB) via `ATTACH … AS old`.
///
/// Call with **no** open write transaction on `conn`’s main DB (avoids `database old is locked`).
fn copy_previous_aux_tables(conn: &Connection, live_db_path: &Path) -> Result<(), anyhow::Error> {
    if !live_db_path.exists() {
        return Ok(());
    }

    let path_str = path_str_for_attach(live_db_path)?;
    conn.execute(UblxDbStatements::ATTACH_OLD_DB, rusqlite::params![path_str])?;

    let copied = match conn.query_row(UblxDbStatements::SELECT_COUNT_DELTA_LOG_ROWS, [], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(1) => conn.execute(UblxDbStatements::COPY_PREVIOUS_DELTA_LOG, [])?,
        _ => 0,
    };
    if copied > 0 {
        log::debug!("copied {copied} previous delta_log rows into tmp");
    }

    let has_lens = matches!(
        conn.query_row(UblxDbStatements::SELECT_LENS_TABLE_EXISTS, [], |row| row
            .get::<_, i64>(0)),
        Ok(1)
    );
    if has_lens {
        conn.execute(UblxDbStatements::COPY_PREVIOUS_PATH, [])?;
        conn.execute(UblxDbStatements::COPY_PREVIOUS_LENS, [])?;
        let n = conn.execute(UblxDbStatements::COPY_PREVIOUS_LENS_PATH, [])?;
        log::debug!("copied previous lens tables into tmp ({n} lens_path rows)");
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
    let created_ns = get_created_ns();

    for delta_type in DeltaType::iter() {
        db_ops_utils::insert_results_into_delta_log_by_type(
            &mut stmt, nefax, diff, delta_type, created_ns,
        )?;
    }

    Ok(())
}

/// Ensure settings table has `config_source` column (migration for existing DBs).
fn ensure_settings_config_source(conn: &Connection) {
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN config_source TEXT", []);
}

/// Load cached settings from the ublx DB. Returns `None` if the settings table is empty (e.g. DB created before settings existed).
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_settings_from_db(db_path: &Path) -> Result<Option<UblxSettings>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    ensure_settings_config_source(&conn);
    let settings_query = UblxDbStatements::create_query_for_settings_from_db();
    conn.query_row(&settings_query, [], |row| {
        let nt: i64 = row.get(0)?;
        Ok(UblxSettings {
            // DB stores i64; clamp negative corrupt values, then convert without lossy cast.
            num_threads: usize::try_from(nt.max(0)).unwrap_or(0),
            drive_type: row.get(1)?,
            parallel_walk: row.get::<_, i64>(2)? != 0,
            config_source: row.get::<_, Option<String>>(3).ok().flatten(),
        })
    })
    .optional()
    .map_err(Into::into)
}

/// True if `snapshot` has at least one row. Lets `main` treat “empty snapshot + no local `ublx.toml`” as first-run even when the `.ublx` file already exists (e.g. quit before the prompt finished).
#[must_use]
pub fn snapshot_has_any_row(db_path: &Path) -> bool {
    if !db_path.exists() {
        return false;
    }
    db_ops_utils::snapshot_table_has_rows(db_path)
}

/// Load (path, `zahir_json`) from the snapshot table for paths that have non-empty `zahir_json`. Use when reusing prior zahir for unchanged mtime. Empty if DB missing or table empty.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
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
        out.insert(normalize_snapshot_rel_path_str(&path), json);
    }
    Ok(out)
}

/// Load (`path`, `category`) from the snapshot table. Empty if DB missing. Keeps category stable when only `zahir_json` is updated.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_snapshot_category_map(
    db_path: &Path,
) -> Result<std::collections::HashMap<String, String>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(std::collections::HashMap::new());
    }
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_PATH_CATEGORY)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = std::collections::HashMap::new();
    for r in rows {
        let (path, category) = r?;
        out.insert(normalize_snapshot_rel_path_str(&path), category);
    }
    Ok(out)
}

/// Load prior Nefax from `.ublx` `snapshot` only. Returns `None` when the table is empty (after skipping absolute-path rows).
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_nefax_from_db(
    dir_to_ublx: &Path,
    db_path: &Path,
) -> Result<Option<NefaxResult>, anyhow::Error> {
    db_ops_utils::NefaxFromGivenDB::new(dir_to_ublx, db_path).load_nefax_from_given_db()
}

/// Load distinct categories from the snapshot table for the TUI. Returns empty vec if table missing or empty.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_snapshot_categories(db_path: &Path) -> Result<Vec<String>, anyhow::Error> {
    let conn = open_for_snapshot_tui_read(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_CATEGORIES)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Distinct snapshot timestamps from `delta_log` (`created_ns`), newest first. Empty if table missing or empty.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_delta_log_snapshot_timestamps(db_path: &Path) -> Result<Vec<i64>, anyhow::Error> {
    let conn = open_for_snapshot_tui_read(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_DELTA_LOG_SNAPSHOT_TIMESTAMPS)?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Rows in `delta_log` for a given `delta_type`: (`created_ns`, path), newest snapshot first, then path order.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_delta_log_rows_by_type(
    db_path: &Path,
    delta_type: &str,
) -> Result<Vec<(i64, String)>, anyhow::Error> {
    let conn = open_for_snapshot_tui_read(db_path)?;
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

/// Load snapshot rows (path, category, size) for the TUI. `zahir_json` is not loaded; use [`load_zahir_json_for_path`] for the selected row.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_snapshot_rows_for_tui(
    db_path: &Path,
    category_filter: Option<&str>,
) -> Result<Vec<SnapshotTuiRow>, anyhow::Error> {
    let conn = open_for_snapshot_tui_read(db_path)?;
    let mut out = Vec::new();
    if let Some(cat) = category_filter {
        let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_ROWS_FOR_TUI_BY_CATEGORY)?;
        let rows = stmt.query_map(rusqlite::params![cat], |row| {
            let size: i64 = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                size.max(0).cast_unsigned(),
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
                size.max(0).cast_unsigned(),
            ))
        })?;
        for r in rows {
            out.push(r?);
        }
    }
    Ok(out)
}

/// Load `zahir_json` for a single path (for right-pane content). Returns None if path not found or `zahir_json` is null/empty.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_zahir_json_for_path(
    db_path: &Path,
    path: &str,
) -> Result<Option<String>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(None);
    }
    let conn = open_for_snapshot_tui_read(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_ZAHIR_JSON_BY_PATH)?;
    let opt: Option<String> = stmt
        .query_row(rusqlite::params![path], |row| row.get::<_, String>(0))
        .optional()?;
    Ok(opt.filter(|s| !s.is_empty()))
}

/// Load `mtime_ns` for a single path (for viewer footer last-modified). Returns None if path not found.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_mtime_for_path(db_path: &Path, path: &str) -> Result<Option<i64>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(None);
    }
    let conn = open_for_snapshot_tui_read(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_MTIME_BY_PATH)?;
    stmt.query_row(rusqlite::params![path], |row| row.get::<_, i64>(0))
        .optional()
        .map_err(Into::into)
}

/// Row for duplicate detection: path, size, optional hash (32 bytes). Directories excluded by query.
pub type SnapshotPathSizeHash = (String, u64, Option<Vec<u8>>);

/// Load path, size, hash for non-directory snapshot rows (for duplicate grouping).
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` open/query errors.
pub fn load_snapshot_path_size_hash(
    db_path: &Path,
) -> Result<Vec<SnapshotPathSizeHash>, anyhow::Error> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = open_for_snapshot_tui_read(db_path)?;
    let mut stmt = conn.prepare(UblxDbStatements::SELECT_SNAPSHOT_PATH_SIZE_HASH)?;
    let rows = stmt.query_map([], |row| {
        let size: i64 = row.get(1)?;
        Ok((
            row.get::<_, String>(0)?,
            size.max(0).cast_unsigned(),
            row.get::<_, Option<Vec<u8>>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Apply one `ZahirScan` [`ZahirOutput`] to a snapshot row (e.g. space menu "Enhance with `ZahirScan`").
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` errors or JSON serialization failure.
pub fn update_snapshot_zahir_for_path(
    db_path: &Path,
    dir_to_ublx: &Path,
    path_rel: &str,
    output: &ZahirOutput,
) -> Result<(), anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let full_path = dir_to_ublx.join(path_rel);
    let zahir_json = zahir_output_to_json_for_path(Some(output), &full_path, path_rel);
    conn.execute(
        UblxDbStatements::UPDATE_SNAPSHOT_ZAHIR_JSON_ONLY,
        rusqlite::params![zahir_json, path_rel],
    )?;
    Ok(())
}
