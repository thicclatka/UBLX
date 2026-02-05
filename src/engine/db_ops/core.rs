//! Index DB and related files under the root, all keyed by package name (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).

use std::fs;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use super::consts::{DeltaType, UblxDbSchema, UblxDbStatements};
use super::utils;
use crate::config::{UblxPaths, UblxSettings};
use crate::handlers::nefax_ops::{NefaxDiff, NefaxResult};
use crate::handlers::zahir_ops::{ZahirResult, get_zahir_output_by_path};

/// Write nefax + zahir outputs to the snapshot: build DB at `root/.ublx_tmp` (with schema), insert all rows, write settings and delta_log, then rename to `root/.ublx`. Nefaxer already excluded paths per opts; we write everything nefax gives us. Errors from zahir (phase1_failed, phase2_failed) are not written; caller may use them later.
pub fn write_snapshot_to_db(
    dir_to_ublx: &Path,
    nefax: &NefaxResult,
    zahir_result: &ZahirResult,
    diff: &NefaxDiff,
    settings: &UblxSettings,
) -> Result<(), anyhow::Error> {
    let ublx_paths = UblxPaths::new(dir_to_ublx);
    let tmp_path = ublx_paths.tmp();
    let db_path = ublx_paths.db();

    let zahir_output_by_path = get_zahir_output_by_path(zahir_result);

    let conn = Connection::open(&tmp_path)?;
    conn.execute_batch(&UblxDbSchema::create_ublx_db_sql())?;
    let mut stmt = conn.prepare(UblxDbStatements::INSERT_SNAPSHOT)?;

    utils::insert_results_into_snapshot(
        &mut stmt,
        nefax,
        dir_to_ublx,
        Some(&ublx_paths),
        &zahir_output_by_path,
    )?;
    drop(stmt);

    write_settings(&conn, settings)?;
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
        ],
    )?;
    Ok(())
}

fn write_delta_log(
    conn: &Connection,
    nefax: &NefaxResult,
    diff: &NefaxDiff,
) -> Result<(), anyhow::Error> {
    let mut stmt = conn.prepare(UblxDbStatements::INSERT_DELTA_LOG)?;
    let created_ns = utils::get_created_ns();

    for delta_type in DeltaType::iter() {
        utils::insert_results_into_delta_log_by_type(
            &mut stmt, nefax, diff, delta_type, created_ns,
        )?;
    }

    Ok(())
}

/// Load cached settings from the ublx DB. Returns `None` if the settings table is empty (e.g. DB created before settings existed).
pub fn load_settings_from_db(db_path: &Path) -> Result<Option<UblxSettings>, anyhow::Error> {
    let conn = Connection::open(db_path)?;
    let settings_query = UblxDbStatements::create_query_for_settings_from_db();
    conn.query_row(&settings_query, [], |row| {
        Ok(UblxSettings {
            num_threads: row.get::<_, i64>(0)? as usize,
            drive_type: row.get(1)?,
            parallel_walk: row.get::<_, i64>(2)? != 0,
        })
    })
    .optional()
    .map_err(Into::into)
}

/// Load prior Nefax: if `dir_to_ublx/NEFAX_DB` (`.nefaxer`) exists, load from that; otherwise load from the ublx snapshot at `db_path`. Returns `None` when the chosen source is empty.
pub fn load_nefax_from_db(
    dir_to_ublx: &Path,
    db_path: &Path,
) -> Result<Option<NefaxResult>, anyhow::Error> {
    utils::NefaxFromGivenDB::new(dir_to_ublx, db_path).load_nefax_from_given_db()
}
