use anyhow::Result;
use std::{
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

/// Package name from Cargo; used as stem for all index files.
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
/// Name of the Nefaxer DB file.
pub const NEFAX_DB: &str = ".nefaxer";

/// Stable hex string for a path (for cache filenames). Same path => same string.
#[must_use]
pub fn path_to_hex(path: &Path) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// User config directory for ublx. Global config lives here (e.g. `ublx.toml`).
/// - **Unix (macOS, Linux):** `~/.config/ublx`
/// - **Windows:** `%APPDATA%\ublx`
///   Returns `None` if the underlying env (e.g. `HOME`, `APPDATA`) is not set.
fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join(PKG_NAME))
    }
    #[cfg(not(windows))]
    {
        env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".config").join(PKG_NAME))
    }
}

/// User cache/data directory for ublx.
/// - **Unix (macOS, Linux):** `~/.local/share/ublx`
/// - **Windows:** `%LOCALAPPDATA%\ublx`
///   Returns `None` if the underlying env (e.g. `HOME`, `LOCALAPPDATA`) is not set.
fn cache_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join(PKG_NAME))
    }
    #[cfg(not(windows))]
    {
        env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".local").join("share").join(PKG_NAME))
    }
}

/// Path to the global config file: `config_dir()/ublx.toml`. `None` if [`config_dir`] is unavailable.
#[must_use]
pub fn global_config_toml() -> Option<PathBuf> {
    config_dir().map(|c| c.join(format!("{PKG_NAME}.toml")))
}

/// Path to the cached "last applied" config for this dir: `cache_dir()/configs/[path_hex].toml`.
/// Per-indexed-dir so global + local overlay is cached by path. Fallback when hot reload gets invalid config.
#[must_use]
pub fn last_applied_config_path(dir: &Path) -> Option<PathBuf> {
    cache_dir().map(|c| c.join("configs").join(format!("{}.toml", path_to_hex(dir))))
}

/// Paths for the index DB and related files under an indexed `dir_to_ublx_abs`. All names use `PKG_NAME` (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).
#[derive(Clone, Debug)]
pub struct UblxPaths {
    pub dir_to_ublx_abs: PathBuf,
}

impl UblxPaths {
    #[must_use]
    pub fn new(dir_to_ublx: &Path) -> Self {
        Self {
            dir_to_ublx_abs: dir_to_ublx.to_path_buf(),
        }
    }

    #[must_use]
    pub fn log_path(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!("{PKG_NAME}.log"))
    }

    /// Hidden config path: `dir_to_ublx_abs/.ublx.toml`.
    #[must_use]
    pub fn hidden_toml(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}.toml"))
    }

    /// Visible config path: `dir_to_ublx_abs/ublx.toml`.
    #[must_use]
    pub fn visible_toml(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!("{PKG_NAME}.toml"))
    }

    /// True if `path` (relative to `dir_to_ublx_abs`) is the hidden or visible ublx config file.
    #[must_use]
    pub fn is_config_file(&self, path: &Path) -> bool {
        let Some(name) = path.file_name() else {
            return false;
        };
        self.hidden_toml().file_name() == Some(name)
            || self.visible_toml().file_name() == Some(name)
    }

    /// Path to the config file to use: checks for `dir_to_ublx_abs/.ublx.toml` then `dir_to_ublx_abs/ublx.toml`; returns the first that exists, or `None`.
    #[must_use]
    pub fn toml_path(&self) -> Option<PathBuf> {
        let hidden = self.hidden_toml();
        let visible = self.visible_toml();
        if hidden.exists() {
            Some(hidden)
        } else if visible.exists() {
            Some(visible)
        } else {
            None
        }
    }

    /// Main DB file. e.g. `dir_to_ublx_abs/.ublx`. `SQLite` creates it if missing.
    #[must_use]
    pub fn db(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}"))
    }

    /// Nefaxer index file (e.g. `dir_to_ublx_abs/.nefaxer`). When present, used as prior snapshot before ublx snapshot.
    #[must_use]
    pub fn nefax_db(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(NEFAX_DB)
    }

    /// Temp file (e.g. write-then-rename). e.g. `dir_to_ublx_abs/.ublx_tmp`.
    #[must_use]
    pub fn tmp(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}_tmp"))
    }

    /// WAL file for [`Self::tmp`] when snapshot build uses `journal_mode=WAL`. e.g. `.ublx_tmp-wal`.
    #[must_use]
    pub fn tmp_wal(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}_tmp-wal"))
    }

    /// Shared-memory file for [`Self::tmp`] in WAL mode. e.g. `.ublx_tmp-shm`.
    #[must_use]
    pub fn tmp_shm(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}_tmp-shm"))
    }

    /// `SQLite` WAL file (created by `SQLite` when WAL mode is on). e.g. `dir_to_ublx_abs/.ublx-wal`.
    #[must_use]
    pub fn wal(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}-wal"))
    }

    /// `SQLite` shared-memory file (WAL mode). e.g. `dir_to_ublx_abs/.ublx-shm`.
    #[must_use]
    pub fn shm(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{PKG_NAME}-shm"))
    }

    /// Paths to exclude from indexing (db, tmp, wal, shm). Returns segment-style names so nefaxer’s exclude (matched per path component) works, e.g. `.ublx`, `.ublx_tmp`.
    #[must_use]
    pub fn exclude(&self) -> Vec<String> {
        [
            self.db(),
            self.tmp(),
            self.tmp_wal(),
            self.tmp_shm(),
            self.wal(),
            self.shm(),
        ]
        .into_iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
    }

    /// Remove tmp, WAL, and SHM files if they exist. No error if any are missing.
    /// Close the DB connection before calling if you use WAL mode.
    ///
    /// # Errors
    ///
    /// Returns [`anyhow::Error`] when removing an existing auxiliary file fails (e.g. I/O permission denied).
    pub fn remove_aux_files(&self) -> Result<(), anyhow::Error> {
        for p in [
            self.tmp(),
            self.tmp_wal(),
            self.tmp_shm(),
            self.wal(),
            self.shm(),
        ] {
            if p.exists() {
                fs::remove_file(&p)?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn global_config(&self) -> Option<PathBuf> {
        global_config_toml()
    }

    /// User cache dir (`~/.local/share/ublx` or Windows equivalent). Used for last-applied config and future hot-reload fallback.
    #[allow(dead_code)]
    #[must_use]
    pub fn cache_dir(&self) -> Option<PathBuf> {
        cache_dir()
    }

    #[must_use]
    pub fn last_applied_config_path(&self) -> Option<PathBuf> {
        last_applied_config_path(&self.dir_to_ublx_abs)
    }
}

#[must_use]
pub fn get_log_path(dir_to_ublx: &Path) -> PathBuf {
    UblxPaths::new(dir_to_ublx).log_path()
}

#[must_use]
/// Normalize a path string for policy matching (e.g. `photos/vacation` → `photos/vacation`)
pub fn normalize_rel_path_for_policy(s: &str) -> String {
    let s = s.replace('\\', "/");
    let s = s.trim_start_matches("./");
    s.trim_end_matches('/').to_string()
}

/// True if `rel` (relative path) is under or equal to `prefix` (e.g. `photos/vacation` is under `photos`).
#[must_use]
pub fn path_is_under_or_equal(rel: &str, prefix: &str) -> bool {
    rel == prefix || (rel.starts_with(prefix) && rel.as_bytes().get(prefix.len()) == Some(&b'/'))
}
