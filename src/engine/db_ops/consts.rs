use std::fs;
use std::path::Path;

use crate::config::UblxPaths;
use crate::handlers::zahir_ops::ZahirFileType;

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

    /// Update category and zahir_json for an existing snapshot row (streaming write).
    pub const UPDATE_SNAPSHOT_ZAHIR: &'static str =
        "UPDATE snapshot SET category = ?1, zahir_json = ?2 WHERE path = ?3";

    pub const INSERT_SETTINGS: &'static str = "INSERT OR REPLACE INTO settings (id, num_threads, drive_type, parallel_walk) VALUES (1, ?1, ?2, ?3)";

    pub const INSERT_DELTA_LOG: &'static str = "INSERT INTO delta_log (created_ns, path, mtime_ns, size, hash, delta_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

    pub const COPY_PREVIOUS_DELTA_LOG: &'static str = "INSERT INTO main.delta_log (created_ns, path, mtime_ns, size, hash, delta_type) SELECT created_ns, path, mtime_ns, size, hash, delta_type FROM old.delta_log";

    pub const ATTACH_OLD_DB: &'static str = "ATTACH DATABASE ?1 AS old";

    pub const DETACH_OLD_DB: &'static str = "DETACH DATABASE old";

    pub const SELECT_CREATED_NS_FROM_DELTA_LOG: &'static str =
        "SELECT created_ns FROM delta_log ORDER BY created_ns DESC LIMIT 1";

    /// Count rows in delta_log for a given created_ns and delta_type. Params: ?1 = created_ns, ?2 = delta_type.
    pub const SELECT_COUNT_DELTA_LOG_BY_NS_AND_TYPE: &'static str =
        "SELECT COUNT(*) FROM delta_log WHERE created_ns = ?1 AND delta_type = ?2";

    pub const SELECT_COUNT_DELTA_LOG_ROWS: &'static str =
        "SELECT COUNT(*) FROM old.sqlite_master WHERE type='table' AND name='delta_log'";

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

/// Category for ublx db: ublx-defined variants plus all [ZahirFileType] via [UblxDbCategory::Zahir].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UblxDbCategory {
    UblxSettings,
    UblxLog,
    Git,
    Hidden,
    Directory,
    File,
    /// All zahirscan file types; use [ZahirFileType::as_metadata_name] for the display string.
    Zahir(ZahirFileType),
}

impl UblxDbCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            UblxDbCategory::UblxSettings => "UBLX Settings",
            UblxDbCategory::UblxLog => "UBLX Log",
            UblxDbCategory::Git => "Git",
            UblxDbCategory::Hidden => "Hidden",
            UblxDbCategory::Directory => "Directory",
            UblxDbCategory::File => "File",
            UblxDbCategory::Zahir(ft) => ft.as_metadata_name(),
        }
    }

    /// Get the category for a given path.
    pub fn get_category_for_path(
        path_ref: &Path,
        ublx_paths: Option<&UblxPaths>,
        zahir_file_type: Option<&str>,
    ) -> String {
        // Check for ublx.toml
        if ublx_paths.is_some_and(|p| p.is_config_file(path_ref)) {
            return UblxDbCategory::UblxSettings.as_str().to_string();
        }
        // Check for ublx.log
        if ublx_paths.is_some_and(|p| p.log_path() == path_ref) {
            return UblxDbCategory::UblxLog.as_str().to_string();
        }
        // Check for .git
        if Self::is_git_path(path_ref) {
            return UblxDbCategory::Git.as_str().to_string();
        }
        // Check for hidden files
        if Self::is_hidden_path(path_ref) {
            return UblxDbCategory::Hidden.as_str().to_string();
        }
        // Get zahir file type or fallback
        Self::get_zahir_file_type_or_fallback(zahir_file_type, path_ref)
    }

    fn get_zahir_file_type_or_fallback(zahir_file_type: Option<&str>, path_ref: &Path) -> String {
        let fallback = Self::determine_fallback_category(path_ref);
        let unknown = UblxDbCategory::Zahir(ZahirFileType::Unknown).as_str();
        zahir_file_type
            .and_then(|s| (!s.eq_ignore_ascii_case(unknown)).then(|| s.to_string()))
            .unwrap_or(fallback)
    }

    #[inline]
    fn is_hidden_path(path_ref: &Path) -> bool {
        path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("."))
    }

    #[inline]
    fn is_git_path(path_ref: &Path) -> bool {
        path_ref.to_string_lossy().contains(".git")
    }

    #[inline]
    fn is_directory_path(path_ref: &Path) -> bool {
        fs::metadata(path_ref).map(|m| m.is_dir()).unwrap_or(false)
    }

    #[inline]
    fn determine_fallback_category(path_ref: &Path) -> String {
        let is_dir = Self::is_directory_path(path_ref);
        if is_dir {
            UblxDbCategory::Directory.as_str().to_string()
        } else {
            UblxDbCategory::File.as_str().to_string()
        }
    }
}
