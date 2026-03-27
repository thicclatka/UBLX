//! Path helpers (extensions, etc.).

use std::fs;
use std::path::{Path, PathBuf};

/// Resolve a path string from the DB or snapshot against `base` when relative, or use it as-is when absolute.
///
/// Same behavior as [`Path::join`]: if `path` is absolute, it replaces the prefix under `base`.
#[must_use]
pub fn resolve_under_root(base: &Path, path: &str) -> PathBuf {
    base.join(path)
}

/// True if `path` relative to `root` exists on disk and is a directory (`fs::metadata` / `is_dir`).
/// Matches how snapshot rows get category `"Directory"` (see `db_ops` category fallback).
#[must_use]
pub fn rel_path_is_directory(root: &Path, path: &Path) -> bool {
    fs::metadata(root.join(path))
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

/// Path as a string with `/` separators (TOML paths, policy prefix checks, DB keys, cross-platform snapshot maps).
///
/// On Windows, normalizes `\\` to `/` so comparisons match Unix-style config and stored strings.
#[must_use]
pub fn path_to_slash_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Normalize a snapshot `path` column so it matches nefaxer’s relative path strings (`rel_str` / map keys).
///
/// Trims, strips a leading `./` or `.\`, then replaces `\` with `/`.
#[must_use]
pub fn normalize_snapshot_rel_path_str(path: &str) -> String {
    let mut s = path.trim();
    s = s.strip_prefix("./").unwrap_or(s);
    if let Some(rest) = s.strip_prefix(".\\") {
        s = rest;
    }
    s.replace('\\', "/")
}

/// [`PathBuf`] key for nefax-style maps, from a snapshot `path` column (see [`normalize_snapshot_rel_path_str`]).
#[must_use]
pub fn snapshot_rel_path_buf(path_str: &str) -> PathBuf {
    PathBuf::from(normalize_snapshot_rel_path_str(path_str))
}

/// True if `path`'s file extension equals any of `exts` (ASCII case-insensitive, OR semantics).
#[must_use]
pub fn path_has_extension(path: &str, exts: &[&str]) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| exts.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

/// Define a `fn name(path: &str) -> bool` that checks the path suffix against a fixed extension list.
///
/// # Example
///
/// ```ignore
/// define_path_ext_predicate! {
///     #[must_use]
///     pub fn is_markdown_path(path: &str) -> bool {
///         "md", "markdown"
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_path_ext_predicate {
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident($path:ident: &str) -> bool {
            $($ext:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis fn $name(path: &str) -> bool {
            $crate::utils::path_has_extension(path, &[$($ext),+])
        }
    };
}
