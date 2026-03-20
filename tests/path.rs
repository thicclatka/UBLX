//! Path helpers: extension checks and markdown path predicate.

use ublx::render::viewers::markdown::is_markdown_path;
use ublx::utils::path_has_extension;

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
