//! Integration tests: [`ublx::config::paths`] welcome gate + [`ublx::render::path_lines`] wrapping.
//!
//! Settings overlay tests: `tests/settings_overlay.rs`.

use ublx::config::should_show_initial_prompt;
use ublx::render::path_lines::wrap_path_string_segments;

// --- `config::paths` / `should_show_initial_prompt` ------------------------------------------------

#[test]
fn never_in_snapshot_only_mode() {
    assert!(!should_show_initial_prompt(true, false));
    assert!(!should_show_initial_prompt(true, true));
}

#[test]
fn initial_prompt_only_if_no_ubli_db_when_not_snapshot_only() {
    assert!(should_show_initial_prompt(false, false));
    assert!(!should_show_initial_prompt(false, true));
}

// --- `render::path_lines` / `wrap_path_string_segments` --------------------------------------------

#[test]
fn empty_yields_one_empty_line() {
    assert_eq!(wrap_path_string_segments("", 10, "  "), vec![String::new()]);
}

#[test]
fn short_fits_one_line() {
    assert_eq!(
        wrap_path_string_segments("a/b", 80, "  "),
        vec!["a/b".to_string()]
    );
}

#[test]
fn no_separator_hard_wraps_at_width() {
    let s = "abcdefghij";
    let lines = wrap_path_string_segments(s, 4, "  ");
    assert_eq!(
        lines,
        vec![
            "abcd".to_string(),
            "  ef".to_string(),
            "  gh".to_string(),
            "  ij".to_string(),
        ]
    );
}

#[test]
fn prefers_break_after_last_separator_in_window() {
    let s = "alpha/beta";
    let lines = wrap_path_string_segments(s, 6, "  ");
    assert_eq!(lines, vec!["alpha/".to_string(), "  beta".to_string()]);
}
