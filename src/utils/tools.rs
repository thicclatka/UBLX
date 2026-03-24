use colored::Colorize;
use log::{Level, debug, error};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::PKG_NAME;
use crate::utils::exit_error;

/// Binary prefixes (1024-based), shared with [`format_bytes`].
pub const KIB: u64 = 1024;
pub const MIB: u64 = KIB * 1024;
pub const GIB: u64 = MIB * 1024;

/// Half a mebibyte (512 KiB): max single-file text load in the viewer and minimum file size for
/// off-thread image decode — keep these policies in sync unless intentionally diverging.
pub const HALF_MIB_BYTES: u64 = MIB / 2;

/// [`HALF_MIB_BYTES`] as [`usize`] for allocation caps (`HALF_MIB_BYTES` always fits in `usize`).
pub const HALF_MIB_BYTES_USIZE: usize = HALF_MIB_BYTES as usize;

/// Expand a leading `~/` using `HOME` so `cargo run -- ~/src/proj` works (the shell often does not expand `~` in argv).
#[must_use]
pub fn expand_home_dir_arg(path: &Path) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path.to_path_buf();
    };
    let Some(rest) = s.strip_prefix("~/") else {
        return path.to_path_buf();
    };
    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(user) = std::env::var("USERPROFILE") {
            return PathBuf::from(user).join(rest);
        }
    }
    path.to_path_buf()
}

/// Validate that a path is a directory and return the canonicalized path.
/// Symlinks are resolved (e.g. `~/Dropbox` → `~/Library/CloudStorage/...` on macOS).
#[must_use]
pub fn validate_dir(path: &std::path::Path) -> PathBuf {
    let path = expand_home_dir_arg(path);
    if path.exists() && !path.is_dir() {
        error!("'{}' is not a directory", path.display());
        exit_error();
    }
    if !path.exists() {
        error!("'{}' no such file or directory", path.display());
        exit_error();
    }
    path.canonicalize().unwrap_or_else(|e| {
        error!("cannot canonicalize '{}': {}", path.display(), e);
        exit_error();
    })
}

#[must_use]
pub fn canonicalize_dir_to_ublx(dir_to_ublx: &Path) -> PathBuf {
    dir_to_ublx
        .canonicalize()
        .unwrap_or_else(|_| dir_to_ublx.to_path_buf())
}

/// Color the level of the log message.
fn level_colored(level: Level) -> String {
    let s = format!("{level}");
    match level {
        Level::Error => s.red().to_string(),
        Level::Warn => s.yellow().to_string(),
        Level::Info => s.green().to_string(),
        Level::Debug => s.cyan().to_string(),
        Level::Trace => s.dimmed().to_string(),
    }
}

/// Color the path of the log message.
fn path_colored(target: &str) -> String {
    if target.contains("zahirscan") {
        target.bright_green().to_string()
    } else if target.contains("nefaxer") {
        target.bright_cyan().to_string()
    } else {
        target.magenta().to_string()
    }
}

/// Build a logger for test mode without the TUI.
pub fn build_logger_test_mode_no_tui() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .format(|buf, record| {
            let ublx = PKG_NAME.to_uppercase().magenta().bold().to_string();
            let level = level_colored(record.level());
            let path = path_colored(record.target());
            writeln!(buf, "[{} {} {}] {}", ublx, level, path, record.args())
        })
        .init();
    debug!("UBLX test mode logger enabled");
}

/// Format byte count as "B", "KB", "MB", "GB" etc. (uses [`KIB`], [`MIB`], [`GIB`]).
#[must_use]
pub fn format_bytes(n: u64) -> String {
    if n < KIB {
        format!("{n} B")
    } else if n < MIB {
        format!("{:.2} KB", n as f64 / KIB as f64)
    } else if n < GIB {
        format!("{:.2} MB", n as f64 / MIB as f64)
    } else {
        format!("{:.2} GB", n as f64 / GIB as f64)
    }
}
