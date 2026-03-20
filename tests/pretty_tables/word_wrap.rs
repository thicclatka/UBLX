use ublx::render::viewers::pretty_tables::{word_wrap_cell_text, word_wrap_text};

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
fn word_wrap_cell_splits_isodate_on_hyphens_not_mid_digit() {
    // Without hyphens as break opportunities, `word_wrap_text` slices "2025-10-25" every 4 chars.
    let lines = word_wrap_cell_text("2025-10-25", 4);
    assert_eq!(lines, vec!["2025", "-10", "-25"]);
}

#[test]
fn word_wrap_cell_hyphen_long_word_fallback() {
    let lines = word_wrap_cell_text("abcdefghij", 4);
    assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
}
