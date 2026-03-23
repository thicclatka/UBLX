//! Path helpers (extensions, etc.).

use std::path::{Path, PathBuf};

/// Resolve a path string from the DB or snapshot against `base` when relative, or use it as-is when absolute.
///
/// Same behavior as [`Path::join`]: if `path` is absolute, it replaces the prefix under `base`.
#[must_use]
pub fn resolve_under_root(base: &Path, path: &str) -> PathBuf {
    base.join(path)
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
        $vis fn $name($path: &str) -> bool {
            $crate::utils::path::path_has_extension($path, &[$($ext),+])
        }
    };
}
