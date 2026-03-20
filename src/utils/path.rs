//! Path helpers (extensions, etc.).

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
