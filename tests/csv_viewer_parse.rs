//! Delimiter parsing for the right-pane table viewer (zahirscan-aligned hints).

use ublx::render::viewers::csv_handler::parse_csv;

#[test]
fn path_hint_tsv_uses_tab() {
    let raw = "a\tb\nc\td\n";
    let rows = parse_csv(raw, Some("dir/file.tsv")).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["c", "d"]]);
}

#[test]
fn path_hint_psv_uses_pipe() {
    let raw = "a|b\nc|d\n";
    let rows = parse_csv(raw, Some("data.psv")).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["c", "d"]]);
}

#[test]
fn comma_without_hint_still_parses() {
    let raw = "a,b\n1,2\n";
    let rows = parse_csv(raw, None).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
}
