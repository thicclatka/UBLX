use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Package name from Cargo; used as stem for all index files.
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// Name of the Nefaxer DB file.
pub const NEFAX_DB: &str = ".nefaxer";

/// Paths for the index DB and related files under an indexed dir_to_ublx_abs. All names use `PKG_NAME` (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).
#[derive(Clone, Debug)]
pub struct UblxPaths {
    pub dir_to_ublx_abs: PathBuf,
}

impl UblxPaths {
    pub fn new(dir_to_ublx: &Path) -> Self {
        Self {
            dir_to_ublx_abs: dir_to_ublx.to_path_buf(),
        }
    }

    pub fn log_path(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!("{}.log", PKG_NAME))
    }

    /// Hidden config path: `dir_to_ublx_abs/.ublx.toml`.
    pub fn hidden_toml(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{}.toml", PKG_NAME))
    }

    /// Visible config path: `dir_to_ublx_abs/ublx.toml`.
    pub fn visible_toml(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!("{}.toml", PKG_NAME))
    }

    /// True if `path` (relative to dir_to_ublx_abs) is the hidden or visible ublx config file.
    pub fn is_config_file(&self, path: &Path) -> bool {
        let name = match path.file_name() {
            Some(n) => n,
            None => return false,
        };
        self.hidden_toml().file_name() == Some(name)
            || self.visible_toml().file_name() == Some(name)
    }

    /// Path to the config file to use: checks for `dir_to_ublx_abs/.ublx.toml` then `dir_to_ublx_abs/ublx.toml`; returns the first that exists, or `None`.
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

    /// Main DB file. e.g. `dir_to_ublx_abs/.ublx`. SQLite creates it if missing.
    pub fn db(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{}", PKG_NAME))
    }

    /// Nefaxer index file (e.g. `dir_to_ublx_abs/.nefaxer`). When present, used as prior snapshot before ublx snapshot.
    pub fn nefax_db(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(NEFAX_DB)
    }

    /// Temp file (e.g. write-then-rename). e.g. `dir_to_ublx_abs/.ublx_tmp`.
    pub fn tmp(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{}_tmp", PKG_NAME))
    }

    /// SQLite WAL file (created by SQLite when WAL mode is on). e.g. `dir_to_ublx_abs/.ublx-wal`.
    pub fn wal(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{}-wal", PKG_NAME))
    }

    /// SQLite shared-memory file (WAL mode). e.g. `dir_to_ublx_abs/.ublx-shm`.
    pub fn shm(&self) -> PathBuf {
        self.dir_to_ublx_abs.join(format!(".{}-shm", PKG_NAME))
    }

    /// Paths to exclude from indexing (db, tmp, wal, shm). Returns segment-style names so nefaxer’s exclude (matched per path component) works, e.g. `.ublx`, `.ublx_tmp`.
    pub fn exclude(&self) -> Vec<String> {
        [self.db(), self.tmp(), self.wal(), self.shm()]
            .into_iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect()
    }

    /// Remove tmp, WAL, and SHM files if they exist. No error if any are missing.
    /// Close the DB connection before calling if you use WAL mode.
    pub fn remove_aux_files(&self) -> Result<(), anyhow::Error> {
        for p in [self.tmp(), self.wal(), self.shm()] {
            if p.exists() {
                fs::remove_file(&p)?;
            }
        }
        Ok(())
    }
}

pub fn get_log_path(dir_to_ublx: &Path) -> PathBuf {
    UblxPaths::new(dir_to_ublx).log_path()
}
