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
