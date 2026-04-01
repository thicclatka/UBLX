//! In-pane find: literal ranges, scroll, highlights.

use ublx::layout::setup::{UblxState, ViewerFindState};
use ublx::modules::viewer_find::{
    self, highlight_cell_line, highlight_table_cell_line, line_byte_to_index, literal_match_ranges,
    option_needle_nonempty,
};

#[test]
fn literal_match_ranges_empty_needle() {
    assert!(literal_match_ranges("hello world", "").is_empty());
    assert!(literal_match_ranges("hello world", "   ").is_empty());
}

#[test]
fn literal_match_ranges_no_matches() {
    assert!(literal_match_ranges("abc", "z").is_empty());
}

#[test]
fn literal_match_ranges_multiple() {
    assert_eq!(literal_match_ranges("abab", "ab"), vec![(0, 2), (2, 4)]);
}

#[test]
fn line_byte_to_index_empty_haystack() {
    assert_eq!(line_byte_to_index("", 0), 0);
    assert_eq!(line_byte_to_index("", 5), 0);
}

#[test]
fn line_byte_to_index_newlines() {
    let s = "a\nb\nc";
    assert_eq!(line_byte_to_index(s, 0), 0);
    assert_eq!(line_byte_to_index(s, 2), 1);
    assert_eq!(line_byte_to_index(s, 4), 2);
}

#[test]
fn scroll_preview_to_current_first_last_viewport_1() {
    let hay = "line0\nline1\nline2\nline3";
    let mut state = UblxState::new();
    state.viewer_find = ViewerFindState {
        ranges: vec![(0, 4)], // "line" at start of line0
        current: 0,
        ..Default::default()
    };
    viewer_find::scroll_preview_to_current(&mut state, hay, 1);
    assert_eq!(state.panels.preview_scroll, 0);

    state.viewer_find.ranges = vec![(hay.len().saturating_sub(5), hay.len())];
    state.viewer_find.current = 0;
    viewer_find::scroll_preview_to_current(&mut state, hay, 1);
    // last line index 3, v/2 = 0 -> scroll 3
    assert_eq!(state.panels.preview_scroll, 3);
}

#[test]
fn scroll_preview_to_current_viewport_centers() {
    let hay = (0..10)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let line5_start = hay
        .match_indices('\n')
        .nth(4)
        .map(|(i, _)| i + 1)
        .unwrap_or(0);
    let mut state = UblxState::new();
    state.viewer_find.ranges = vec![(line5_start, line5_start + 4)];
    state.viewer_find.current = 0;
    viewer_find::scroll_preview_to_current(&mut state, &hay, 4);
    let line = line_byte_to_index(&hay, line5_start);
    let expected = line.saturating_sub(4 / 2);
    assert_eq!(state.panels.preview_scroll, expected);
}

#[test]
fn highlight_table_cell_line_match_inside_line() {
    let line = highlight_table_cell_line("foo bar baz", 10, &[(13, 16)], 0);
    assert!(line.spans.len() > 1, "expected highlighted segment split");
}

#[test]
fn highlight_cell_line_two_occurrences() {
    let line = highlight_cell_line("a x a x", "x");
    assert!(line.spans.len() >= 3);
}

#[test]
fn highlighted_body_empty_ranges_is_raw() {
    let mut state = UblxState::new();
    state.viewer_find.ranges = vec![];
    let t = viewer_find::highlighted_body(&state, "one\ntwo");
    assert_eq!(ublx::render::panes::text_to_plain_string(&t), "one\ntwo");
}

#[test]
fn viewer_find_state_predicates() {
    let mut vf = ViewerFindState::default();
    assert!(!vf.needle_nonempty());
    assert!(!vf.find_affects_view());
    assert!(!vf.title_bottom_visible());

    vf.query = "  hi  ".to_string();
    assert!(vf.needle_nonempty());
    assert!(vf.title_bottom_visible());
    assert!(!vf.find_affects_view());

    vf.active = true;
    assert!(vf.find_affects_view());
}

#[test]
fn option_needle_nonempty_trims() {
    assert!(!option_needle_nonempty(None));
    assert!(!option_needle_nonempty(Some("")));
    assert!(!option_needle_nonempty(Some("  ")));
    assert!(option_needle_nonempty(Some(" x ")));
}
