//! Comfy-table viewer helpers: word wrap and multiline grid layout.

use ublx::render::viewers::pretty_tables::{prepare_multiline_grid, word_wrap_text};

/// Line count for comfy-table cells joined with `\n`. `str::lines()` skips a final empty line after a trailing `\n`, which would mismatch our padding.
fn cell_visual_lines(s: &str) -> usize {
    s.split('\n').count()
}

#[test]
fn word_wrap_breaks_long_word() {
    let lines = word_wrap_text("abcdefghij", 4);
    assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
}

#[test]
fn word_wrap_spaces() {
    let lines = word_wrap_text("aa bb cc", 5);
    assert_eq!(lines, vec!["aa bb", "cc"]);
}

#[test]
fn short_column_skips_wrap_but_pads() {
    let header = vec!["Long header that wraps".into(), "B".into()];
    let body = vec![vec![
        "first body line is long enough to wrap here yes".into(),
        "x".into(),
    ]];
    // Narrow pane so column 0’s wrap budget is smaller than the header (proportional widths).
    let (h, b) = prepare_multiline_grid(&header, &body, 24);
    assert!(h[0].contains('\n'), "first col should wrap: {:?}", h);
    let header_h = cell_visual_lines(&h[0]);
    assert_eq!(
        cell_visual_lines(&h[1]),
        header_h,
        "short header cell is not word-wrapped but padded to row height: {:?}",
        h
    );
    assert_eq!(h[1].split('\n').next(), Some("B"));
    assert!(
        h[1].split('\n').skip(1).all(|l| l.is_empty()),
        "padding lines after short header text should be blank: {:?}",
        h[1]
    );
    let row0 = &b[0];
    assert!(row0[0].contains('\n'));
    let n0 = cell_visual_lines(&row0[0]);
    assert_eq!(
        cell_visual_lines(&row0[1]),
        n0,
        "short column padded to same visual height: {:?}",
        row0[1]
    );
    assert_eq!(row0[1].split('\n').next(), Some("x"));
}
