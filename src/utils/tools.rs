use colored::Colorize;
use log::{Level, debug, error};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::UBLX_NAMES;
use crate::integrations::ZahirFT;
use crate::utils::exit_error;

/// Binary prefixes (1024-based), shared with [`format_bytes`].
pub const KIB: u64 = 1024;
pub const MIB: u64 = KIB * 1024;
pub const GIB: u64 = MIB * 1024;

/// Half a megabyte (512 KiB): max single-file text load in the viewer and minimum file size for
/// off-thread image decode — keep these policies in sync unless intentionally diverging.
pub const HALF_MIB_BYTES: u64 = MIB / 2;

/// [`HALF_MIB_BYTES`] as [`usize`] for allocation caps (`HALF_MIB_BYTES` always fits in `usize`).
pub const HALF_MIB_BYTES_USIZE: usize = HALF_MIB_BYTES as usize;

/// [`std::fs::Metadata::len`] is `u64`; saturates at `usize::MAX` on 32-bit. Safe for `.min(small_cap)`:
/// the cap (e.g. [`HALF_MIB_BYTES_USIZE`]) still bounds allocation.
#[inline]
#[must_use]
pub fn u64_to_usize_saturating(len: u64) -> usize {
    usize::try_from(len).unwrap_or(usize::MAX)
}

/// Chunk size for [`is_likely_binary`] (first bytes read to detect NUL / invalid UTF-8).
const BINARY_CHECK_CHUNK: usize = 8192;

/// Quick binary heuristic: read the first chunk; binary if NUL present or invalid UTF-8.
#[must_use]
pub fn is_likely_binary(path: &Path) -> bool {
    let Ok(mut f) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; BINARY_CHECK_CHUNK];
    let n = f.read(&mut buf).unwrap_or(0);
    let buf = &buf[..n];
    buf.contains(&0) || std::str::from_utf8(buf).is_err()
}

/// Label for a binary file: `"EXT file"` if the path has an extension (e.g. `"PNG file"`), else `"binary file"`.
#[must_use]
pub fn binary_file_label(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).map_or_else(
        || "binary file".to_string(),
        |ext| format!("{} file", ext.to_uppercase()),
    )
}

/// Read text for a viewer pane: not found, empty for image/PDF (raster preview elsewhere), binary short label, or capped UTF-8 with truncation notice.
///
/// Cap is [`HALF_MIB_BYTES_USIZE`]. When `zahir_type` is [`FileType::Image`], [`FileType::Pdf`], or [`FileType::Video`], returns empty string for a normal file so the render layer loads the preview.
#[must_use]
pub fn file_content_for_viewer(path: &Path, zahir_type: Option<ZahirFT>) -> Option<String> {
    let Ok(meta) = fs::metadata(path) else {
        return Some("(file not found)".to_string());
    };
    if meta.is_file()
        && matches!(
            zahir_type,
            Some(ZahirFT::Image | ZahirFT::Pdf | ZahirFT::Video)
        )
    {
        return Some(String::new());
    }
    if meta.is_file() && is_likely_binary(path) {
        return Some(binary_file_label(path));
    }
    let f = fs::File::open(path).ok()?;
    let cap = HALF_MIB_BYTES_USIZE.min(u64_to_usize_saturating(meta.len()));
    let mut buf = Vec::with_capacity(cap);
    let n = f.take(HALF_MIB_BYTES).read_to_end(&mut buf).ok()?;
    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
    // `take(HALF_MIB_BYTES)` bounds `n` to at most [`HALF_MIB_BYTES`], which fits in `u64`.
    let n_u64 = n as u64;
    let out = if n_u64 >= meta.len() {
        s
    } else {
        format!("{}\n\n… (truncated, {} bytes total)", s, meta.len())
    };
    Some(out)
}

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

/// Like [`validate_dir`] but returns `Err` instead of exiting (e.g. first-run path input).
///
/// # Errors
///
/// Returns `Err` with a message if the path does not exist, is not a directory, or cannot be canonicalized.
pub fn try_validate_dir(path: &Path) -> Result<PathBuf, String> {
    let path = expand_home_dir_arg(path);
    if !path.exists() {
        return Err(format!("no such file or directory: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("not a directory: {}", path.display()));
    }
    path.canonicalize()
        .map_err(|e| format!("cannot canonicalize '{}': {e}", path.display()))
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

/// Build a logger for `--snapshot-only` without the TUI.
pub fn build_logger_snapshot_only_no_tui() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .format(|buf, record| {
            let ublx = UBLX_NAMES
                .pkg_name
                .to_uppercase()
                .magenta()
                .bold()
                .to_string();
            let level = level_colored(record.level());
            let path = path_colored(record.target());
            writeln!(buf, "[{} {} {}] {}", ublx, level, path, record.args())
        })
        .init();
    debug!("UBLX snapshot-only logger enabled");
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

/// Unique stamp for temporary files (nanos XOR pid).
#[must_use]
pub fn unique_stamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
        .unwrap_or(0)
        ^ (u64::from(std::process::id()) << 32)
}

/// Current Unix timestamp in nanoseconds.
#[must_use]
pub fn get_created_ns() -> i64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    i64::try_from(nanos).unwrap_or(i64::MAX)
}
