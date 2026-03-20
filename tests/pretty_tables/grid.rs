use ublx::render::viewers::pretty_tables::{
    VIEWER_TABLE_NO_WRAP_COL_MAX_CHARS, prepare_multiline_grid,
};

use super::common::cell_visual_lines;

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

#[test]
fn header_with_spaces_breaks_at_words_not_mid_word() {
    // Column must be in **wrap** mode (`max_lens` > NO_WRAP), or the header stays one line.
    // A 15-char header alone can equal NO_WRAP (15) and skip wrapping — use a longer body cell.
    let header = vec!["Completion Date".into(), "B".into()];
    let body = vec![vec![
        "x".repeat(VIEWER_TABLE_NO_WRAP_COL_MAX_CHARS + 1),
        "y".into(),
    ]];
    let (h, _) = prepare_multiline_grid(&header, &body, 24);
    let lines: Vec<&str> = h[0].split('\n').collect();
    assert!(
        lines.contains(&"Completion"),
        "expected whole word line, got {:?}",
        lines
    );
    assert!(
        lines.contains(&"Date"),
        "expected whole word line, got {:?}",
        lines
    );
    assert!(
        !lines.iter().any(|l| *l == "Comp" || *l == "leti"),
        "should not hard-break inside Completion: {:?}",
        lines
    );
}
