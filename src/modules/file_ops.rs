//! Rename / delete files and folders under the indexed root (space menu → Rename / Delete).

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::UblxPaths;
use crate::engine::db_ops;
use crate::utils::{normalize_snapshot_rel_path_str, path_to_slash_string};

/// Rename `rel` under `root` to `new_basename` (same parent directory). Updates lens `path` rows.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when validation, `rename`, or DB update fails.
pub fn rename_entry_under_root(
    root: &Path,
    db_path: &Path,
    rel: &str,
    new_basename: &str,
) -> Result<String, anyhow::Error> {
    let rel = normalize_snapshot_rel_path_str(rel);
    let new_base = new_basename.trim();
    if new_base.is_empty() {
        return Err(anyhow::anyhow!("name is empty"));
    }
    if new_base.contains('/') || new_base.contains('\\') {
        return Err(anyhow::anyhow!("name must not contain path separators"));
    }
    let p = Path::new(&rel);
    let new_rel: PathBuf = match p.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(new_base),
        _ => PathBuf::from(new_base),
    };
    let new_rel_str = path_to_slash_string(&new_rel);
    if new_rel_str == rel {
        return Err(anyhow::anyhow!("same path"));
    }
    let old_abs = root.join(&rel);
    let new_abs = root.join(&new_rel_str);
    if new_abs.exists() {
        return Err(anyhow::anyhow!("a path already exists with that name"));
    }
    fs::rename(&old_abs, &new_abs)?;
    db_ops::rename_path_string(db_path, &rel, &new_rel_str)?;
    db_ops::record_delta_log_rename(db_path, root, &rel, &new_rel_str)?;
    let ublx_paths = UblxPaths::new(root);
    db_ops::rename_snapshot_row(db_path, root, Some(&ublx_paths), &rel, &new_rel_str)?;
    Ok(new_rel_str)
}

/// Parse bulk-rename buffer from an external editor: one line per path, same order as `old_paths`.
/// Each line is the **new** relative path (forward slashes); must keep the same parent directory as the original.
///
/// Returns `(old_rel, new_basename)` pairs for entries that actually change.
///
/// # Errors
///
/// Returns [`anyhow::Error`] when line count mismatches, a line is empty, or a path would change folders.
pub fn parse_bulk_rename_lines(
    old_paths: &[String],
    editor_file_body: &str,
) -> Result<Vec<(String, String)>, anyhow::Error> {
    let lines: Vec<&str> = editor_file_body.lines().map(str::trim).collect();
    if lines.iter().any(|l| l.is_empty()) {
        return Err(anyhow::anyhow!(
            "remove blank lines — one non-empty path per line, same order as before"
        ));
    }
    if lines.len() != old_paths.len() {
        return Err(anyhow::anyhow!(
            "expected {} lines, got {} (same order as selection)",
            old_paths.len(),
            lines.len()
        ));
    }

    let mut out = Vec::new();
    for (old_raw, line) in old_paths.iter().zip(lines.iter()) {
        let old_n = normalize_snapshot_rel_path_str(old_raw);
        let new_n = normalize_snapshot_rel_path_str(line);
        if old_n == new_n {
            continue;
        }
        let old_p = Path::new(&old_n);
        let new_p = Path::new(&new_n);
        let old_parent = old_p.parent().unwrap_or_else(|| Path::new(""));
        let new_parent = new_p.parent().unwrap_or_else(|| Path::new(""));
        if old_parent != new_parent {
            return Err(anyhow::anyhow!(
                "bulk rename must stay in the same folder: {old_n} → {new_n}"
            ));
        }
        let new_base = new_p
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid new path: {new_n}"))?;
        if new_base.is_empty() {
            return Err(anyhow::anyhow!("empty file name in: {new_n}"));
        }
        out.push((old_n, new_base.to_string()));
    }
    Ok(out)
}

/// Delete file or directory at `rel` under `root`, then remove lens `path` row if present.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on I/O or DB errors.
pub fn delete_entry_under_root(
    root: &Path,
    db_path: &Path,
    rel: &str,
) -> Result<(), anyhow::Error> {
    let rel = normalize_snapshot_rel_path_str(rel);
    let abs = root.join(&rel);
    if abs.is_dir() {
        fs::remove_dir_all(&abs)?;
    } else {
        fs::remove_file(&abs)?;
    }
    db_ops::delete_path_row(db_path, &rel)?;
    db_ops::insert_delta_log_removed(db_path, &rel)?;
    db_ops::delete_snapshot_row(db_path, &rel)?;
    Ok(())
}
