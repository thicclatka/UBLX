//! Path helpers: extension checks, `resolve_under_root`, and markdown path predicate.

use std::path::{Path, PathBuf};

use ublx::render::viewers::markdown::is_markdown_path;
use ublx::utils::{path_has_extension, resolve_under_root};

#[test]
fn path_has_extension_matches_final_segment() {
    assert!(path_has_extension("foo.md", &["md"]));
    assert!(path_has_extension("foo.MD", &["md"]));
    assert!(path_has_extension("a/b/c.markdown", &["markdown"]));
    assert!(path_has_extension("a/b/c.MARKDOWN", &["markdown"]));
}

#[test]
fn path_has_extension_rejects_non_matching() {
    assert!(!path_has_extension("foo.txt", &["md"]));
    assert!(!path_has_extension("foo", &["md"]));
    assert!(!path_has_extension("foo.md.bak", &["md"]));
}

#[test]
fn is_markdown_path_matches_extensions() {
    assert!(is_markdown_path("README.md"));
    assert!(is_markdown_path("notes.MARKDOWN"));
    assert!(is_markdown_path("deep/path/file.md"));
}

#[test]
fn is_markdown_path_rejects_others() {
    assert!(!is_markdown_path("file.txt"));
    assert!(!is_markdown_path("noext"));
    assert!(!is_markdown_path("foo.md.backup"));
}

#[test]
fn resolve_under_root_joins_relative() {
    let base = Path::new("project");
    assert_eq!(
        resolve_under_root(base, "a/b"),
        PathBuf::from("project").join("a/b")
    );
}

#[cfg(unix)]
#[test]
fn resolve_under_root_absolute_replaces_prefix() {
    assert_eq!(
        resolve_under_root(Path::new("/proj/.ublx"), "/x/y"),
        PathBuf::from("/x/y")
    );
}
