use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Package name from Cargo; used as stem for all index files.
pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// Name of the Nefaxer DB file.
pub const NEFAX_DB: &str = ".nefaxer";

/// Paths for the index DB and related files under an indexed root. All names use `PKG_NAME` (e.g. `.ublx`, `.ublx_tmp`, `.ublx-wal`).
#[derive(Clone, Debug)]
pub struct UblxPaths {
    pub root: PathBuf,
}

impl UblxPaths {
    pub fn new(dir_to_ublx: &Path) -> Self {
        Self {
            root: dir_to_ublx.to_path_buf(),
        }
    }

    pub fn log_path(&self) -> PathBuf {
        self.root.join(format!("{}.log", PKG_NAME))
    }

    /// Hidden config path: `root/.ublx.toml`.
    pub fn hidden_toml(&self) -> PathBuf {
        self.root.join(format!(".{}.toml", PKG_NAME))
    }

    /// Visible config path: `root/ublx.toml`.
    pub fn visible_toml(&self) -> PathBuf {
        self.root.join(format!("{}.toml", PKG_NAME))
    }

    /// True if `path` (relative to root) is the hidden or visible ublx config file.
    pub fn is_config_file(&self, path: &Path) -> bool {
        let name = match path.file_name() {
            Some(n) => n,
            None => return false,
        };
        self.hidden_toml().file_name() == Some(name)
            || self.visible_toml().file_name() == Some(name)
    }

    /// Path to the config file to use: checks for `root/.ublx.toml` then `root/ublx.toml`; returns the first that exists, or `None`.
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

    /// Main DB file. e.g. `root/.ublx`. SQLite creates it if missing.
    pub fn db(&self) -> PathBuf {
        self.root.join(format!(".{}", PKG_NAME))
    }

    /// Nefaxer index file (e.g. `root/.nefaxer`). When present, used as prior snapshot before ublx snapshot.
    pub fn nefax_db(&self) -> PathBuf {
        self.root.join(NEFAX_DB)
    }

    /// Temp file (e.g. write-then-rename). e.g. `root/.ublx_tmp`.
    pub fn tmp(&self) -> PathBuf {
        self.root.join(format!(".{}_tmp", PKG_NAME))
    }

    /// SQLite WAL file (created by SQLite when WAL mode is on). e.g. `root/.ublx-wal`.
    #[allow(dead_code)]
    pub fn wal(&self) -> PathBuf {
        self.root.join(format!(".{}-wal", PKG_NAME))
    }

    /// SQLite shared-memory file (WAL mode). e.g. `root/.ublx-shm`.
    #[allow(dead_code)]
    pub fn shm(&self) -> PathBuf {
        self.root.join(format!(".{}-shm", PKG_NAME))
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
    #[allow(dead_code)]
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
