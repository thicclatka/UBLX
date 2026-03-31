//! Integration tests: [`ublx::config::paths`] welcome gate, path-hash helpers, + [`ublx::render::path_lines`] wrapping.
//!
//! Settings overlay tests: `tests/settings_overlay.rs`.

use ublx::config::{
    INDEX_DB_FILE_EXT, PKG_NAME, hash_suffix_from_db_stem, is_hex_hash16,
    should_show_initial_prompt,
};
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

#[test]
fn index_db_file_ext_is_dot_pkg_name() {
    assert_eq!(INDEX_DB_FILE_EXT, concat!(".", env!("CARGO_PKG_NAME")));
    assert_eq!(INDEX_DB_FILE_EXT, format!(".{PKG_NAME}"));
}

#[test]
fn hex_hash16_accepts_16_hex_digits() {
    assert!(is_hex_hash16("729a9c26db109730"));
    assert!(!is_hex_hash16("729a9c26db10973"));
    assert!(!is_hex_hash16("729a9c26db1097300"));
    assert!(!is_hex_hash16("g29a9c26db109730"));
}

#[test]
fn hash_suffix_from_db_stem_parses_stem() {
    assert_eq!(
        hash_suffix_from_db_stem("mydir_729a9c26db109730"),
        Some("729a9c26db109730")
    );
    assert_eq!(hash_suffix_from_db_stem("729a9c26db109730"), None);
    assert_eq!(hash_suffix_from_db_stem("no_hash_here"), None);
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
