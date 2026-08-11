//! Entry path validation and root-relative disk resolve.

use std::path::{Path, PathBuf};

use super::super::error::ApiError;
use super::super::state::canonicalize_dir;

/// Normalize and reject empty / `..` segments (path-traversal).
pub(in crate::cli::serve) fn require_rel_path(path: &str) -> Result<String, ApiError> {
    let path = normalize_entry_path(path);
    if path.is_empty() || path.split('/').any(|s| s == "..") {
        return Err(ApiError::bad_request("invalid entry path"));
    }
    Ok(path)
}

/// Join catalog-relative path under the current root; reject lexical escapes.
///
/// Containment is checked on the **logical** path under `root` (no `..`, already enforced by
/// [`require_rel_path`]). The returned path is canonicalized for open/read and **may** resolve
/// outside `root` when a symlink in the tree points elsewhere — same as TUI viewing and required
/// for `follow_links` snapshots. Do not re-check `canon.starts_with(root)` after canonicalize.
pub(super) fn resolve_entry_disk_path(root: &Path, rel: &str) -> Result<PathBuf, ApiError> {
    let root = canonicalize_dir(root);
    let joined = root.join(rel);
    if !joined.starts_with(&root) {
        return Err(ApiError::bad_request("path escapes project root"));
    }
    joined
        .canonicalize()
        .map_err(|e| ApiError::not_found(format!("file not found for {rel}: {e}")))
}

/// Axum may leave a leading slash on `{*path}`; catalog paths are relative without it.
fn normalize_entry_path(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ublx-serve-paths-{label}-{ns}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_allows_symlink_target_outside_root() {
        let root = temp_dir("root");
        let outside = temp_dir("outside");
        let outside_file = outside.join("secret.txt");
        fs::write(&outside_file, b"hi").unwrap();

        let link = root.join("cloud");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        #[cfg(not(unix))]
        {
            let _ = (root, outside, outside_file, link);
            return;
        }

        let rel = "cloud/secret.txt";
        let abs = match resolve_entry_disk_path(&root, rel) {
            Ok(p) => p,
            Err(e) => panic!("symlink under root should resolve: {e}"),
        };
        assert_eq!(abs, outside_file.canonicalize().unwrap());
        assert!(
            !abs.starts_with(root.canonicalize().unwrap()),
            "canonical target is outside root by design"
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn resolve_rejects_parent_dir_join_quirk() {
        let root = temp_dir("root2");
        // Absolute-looking after join would escape; require_rel_path strips leading `/`,
        // but a raw absolute `rel` must still fail the lexical check if passed here.
        #[cfg(unix)]
        {
            let err = resolve_entry_disk_path(&root, "/etc/passwd").unwrap_err();
            assert!(err.to_string().contains("escapes"), "unexpected: {err}");
        }
        let _ = fs::remove_dir_all(root);
    }
}
