//! Tests for `utils::format` helpers.

use ublx::utils::format::{
    clamp_selection, clamp_selection_opt, format_timestamp_ns, frame_string_with_spaces,
    truncate_middle,
};

#[test]
fn clamp_selection_in_range() {
    assert_eq!(clamp_selection(0, 5), 0);
    assert_eq!(clamp_selection(2, 5), 2);
    assert_eq!(clamp_selection(4, 5), 4);
}

#[test]
fn clamp_selection_over_max() {
    assert_eq!(clamp_selection(5, 5), 4);
    assert_eq!(clamp_selection(10, 3), 2);
}

#[test]
fn clamp_selection_empty_list() {
    assert_eq!(clamp_selection(0, 0), 0);
    assert_eq!(clamp_selection(3, 0), 0);
}

#[test]
fn clamp_selection_opt_some() {
    assert_eq!(clamp_selection_opt(1, 5), Some(1));
    assert_eq!(clamp_selection_opt(4, 5), Some(4));
    assert_eq!(clamp_selection_opt(10, 5), Some(4));
}

#[test]
fn clamp_selection_opt_none() {
    assert_eq!(clamp_selection_opt(0, 0), None);
    assert_eq!(clamp_selection_opt(3, 0), None);
}

#[test]
fn test_frame_string_with_spaces() {
    assert_eq!(frame_string_with_spaces("Delta"), " Delta ");
    assert_eq!(frame_string_with_spaces(""), "  ");
}

#[test]
fn truncate_middle_short() {
    assert_eq!(truncate_middle("short", 10), "short");
    assert_eq!(truncate_middle("ab", 3), "ab");
}

#[test]
fn truncate_middle_long() {
    let s = truncate_middle("hello world", 8);
    assert_eq!(s.len(), 8);
    assert!(s.contains("..."));
}

#[test]
fn format_timestamp_ns_valid() {
    let s = format_timestamp_ns(1_000_000_000); // 1 second after Unix epoch
    assert!(
        !s.contains("invalid"),
        "expected valid timestamp string, got {s:?}"
    );
    assert!(
        s.chars().filter(|c| c.is_ascii_digit()).count() >= 8,
        "expected digits in output: {s:?}"
    );
}

#[test]
fn format_timestamp_ns_negative_no_panic() {
    let s = format_timestamp_ns(-1);
    assert!(!s.is_empty());
}
