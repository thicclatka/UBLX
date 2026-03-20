//! Shared helpers for pretty-table integration tests.

/// Line count for comfy-table cells joined with `\n`. `str::lines()` skips a final empty line
/// after a trailing `\n`, which would mismatch our padding.
pub fn cell_visual_lines(s: &str) -> usize {
    s.split('\n').count()
}
