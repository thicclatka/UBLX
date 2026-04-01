//! Bulk-rename buffer parsing (`parse_bulk_rename_lines`).

use ublx::handlers::applets::file_ops::parse_bulk_rename_lines;

#[test]
fn parse_bulk_two_lines() {
    let old = vec!["a.txt".to_string(), "b.txt".to_string()];
    let body = "x.txt\ny.txt\n";
    let p = parse_bulk_rename_lines(&old, body).unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0], ("a.txt".to_string(), "x.txt".to_string()));
    assert_eq!(p[1], ("b.txt".to_string(), "y.txt".to_string()));
}

#[test]
fn parse_bulk_skips_unchanged() {
    let old = vec!["foo/a.txt".to_string()];
    let body = "foo/a.txt\n";
    let p = parse_bulk_rename_lines(&old, body).unwrap();
    assert!(p.is_empty());
}

#[test]
fn parse_bulk_rejects_folder_change() {
    let old = vec!["foo/a.txt".to_string()];
    let body = "bar/a.txt\n";
    assert!(parse_bulk_rename_lines(&old, body).is_err());
}

#[test]
fn parse_bulk_line_count_mismatch() {
    let old = vec!["a.txt".to_string(), "b.txt".to_string()];
    assert!(parse_bulk_rename_lines(&old, "only.txt\n").is_err());
}
