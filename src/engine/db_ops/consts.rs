use std::fs;
use std::path::Path;

use crate::config::UblxPaths;

/// Schema for the ublx DB
pub struct UblxDbSchema;

impl UblxDbSchema {
    /// Snapshot table: one row per path. Nefaxer columns + ublx category + optional zahirscan JSON.
    pub const CREATE_SNAPSHOT: &'static str = "
CREATE TABLE IF NOT EXISTS snapshot (
    path TEXT PRIMARY KEY,
    mtime_ns INTEGER NOT NULL,
    size INTEGER NOT NULL,
    hash BLOB,
    category TEXT,
    zahir_json TEXT
);
";

    /// Settings table: single row storing disk/tuning so we can skip disk check when .ublx exists.
    pub const CREATE_SETTINGS: &'static str = "
CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    num_threads INTEGER NOT NULL,
    drive_type TEXT NOT NULL,
    parallel_walk INTEGER NOT NULL
);
";

    /// Delta log: one row per change (added, mod, removed); no zahir result, just path + meta.
    pub const CREATE_DELTA_LOG: &'static str = "
CREATE TABLE IF NOT EXISTS delta_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_ns INTEGER NOT NULL,
    path TEXT NOT NULL,
    mtime_ns INTEGER,
    size INTEGER,
    hash BLOB,
    delta_type TEXT NOT NULL CHECK (delta_type IN ('added', 'mod', 'removed'))
);
";

    /// SQL to create all ublx tables (snapshot, settings, delta_log). Use when opening or creating the DB.
    pub fn create_ublx_db_sql() -> String {
        format!(
            "{}{}{}",
            Self::CREATE_SNAPSHOT,
            Self::CREATE_SETTINGS,
            Self::CREATE_DELTA_LOG,
        )
    }
}

/// Statements for the ublx DB
pub struct UblxDbStatements;

impl UblxDbStatements {
    pub const INSERT_SNAPSHOT: &'static str = "INSERT OR REPLACE INTO snapshot (path, mtime_ns, size, hash, category, zahir_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

    pub const INSERT_SETTINGS: &'static str = "INSERT OR REPLACE INTO settings (id, num_threads, drive_type, parallel_walk) VALUES (1, ?1, ?2, ?3)";

    pub const INSERT_DELTA_LOG: &'static str = "INSERT INTO delta_log (created_ns, path, mtime_ns, size, hash, delta_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

    pub fn create_query_for_nefax_from_db(table_name: &str) -> String {
        format!("SELECT path, mtime_ns, size, hash FROM {}", table_name)
    }

    pub fn create_query_for_settings_from_db() -> String {
        "SELECT num_threads, drive_type, parallel_walk FROM settings WHERE id = 1".to_string()
    }
}

/// Delta type for the delta_log table.
#[derive(Clone, Copy, Debug)]
pub enum DeltaType {
    Added,
    Mod,
    Removed,
}

impl DeltaType {
    pub fn as_str(self) -> &'static str {
        match self {
            DeltaType::Added => "added",
            DeltaType::Mod => "mod",
            DeltaType::Removed => "removed",
        }
    }

    pub fn iter() -> impl Iterator<Item = DeltaType> {
        [DeltaType::Added, DeltaType::Mod, DeltaType::Removed].into_iter()
    }
}

/// Category for ublx db
pub struct UblxDbCategory;

impl UblxDbCategory {
    pub const UBLX_SETTINGS: &'static str = "UBLX Settings";
    pub const GIT: &'static str = "Git";
    pub const HIDDEN: &'static str = "Hidden";
    pub const DIRECTORY: &'static str = "Directory";
    pub const FILE: &'static str = "File";

    pub fn get_category_for_path(
        path_ref: &Path,
        ublx_paths: Option<&UblxPaths>,
        zahir_file_type: Option<&str>,
    ) -> String {
        if ublx_paths.is_some_and(|p| p.is_config_file(path_ref)) {
            return UblxDbCategory::UBLX_SETTINGS.to_string();
        }
        let path_str = path_ref.to_string_lossy();
        if path_str.contains(".git") {
            return UblxDbCategory::GIT.to_string();
        }
        if path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            return UblxDbCategory::HIDDEN.to_string();
        }
        let fallback = Self::determine_fallback_category(path_ref);
        zahir_file_type
            .filter(|s| !s.eq_ignore_ascii_case("unknown"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    }

    fn determine_fallback_category(path_ref: &Path) -> String {
        let is_dir = Self::is_directory(path_ref);
        if is_dir {
            UblxDbCategory::DIRECTORY.to_string()
        } else {
            UblxDbCategory::FILE.to_string()
        }
    }

    fn is_directory(path_ref: &Path) -> bool {
        fs::metadata(path_ref).map(|m| m.is_dir()).unwrap_or(false)
    }
}
