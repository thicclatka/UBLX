//! Shared formatting helpers (e.g. timestamps for UI).

use chrono::{DateTime, Local};

/// Float comparison epsilons: different tolerances per domain.
pub struct Epsilon;
impl Epsilon {
    /// Number display: treat fractional part as zero below this (f64).
    pub const FORMAT: f64 = 1e-9;
    /// Color math (RGB/HSL): treat values as equal within this (f32).
    pub const COLOR: f32 = 1e-6;
}

/// If `s` is longer than `max_len`, replace the middle with "..." so the result shows start and end (total length `max_len`). Uses character count.
pub fn truncate_middle(s: &str, max_len: usize) -> String {
    const ELLIPSIS: &str = "...";
    let ellipsis_len = ELLIPSIS.chars().count();
    if max_len <= ellipsis_len || s.chars().count() <= max_len {
        return s.to_string();
    }
    let take_each = (max_len - ellipsis_len) / 2;
    let start: String = s.chars().take(take_each).collect();
    let n = s.chars().count();
    let end: String = s.chars().skip(n.saturating_sub(take_each)).collect();
    format!("{start}{ELLIPSIS}{end}")
}

/// Format Unix timestamp in nanoseconds as local date-time (e.g. "2025-02-06 14:30:00").
pub fn format_timestamp_ns(ns: i64) -> String {
    const NS_PER_S: i64 = 1_000_000_000;
    let secs = ns / NS_PER_S;
    let subsec = ((ns % NS_PER_S) + NS_PER_S) % NS_PER_S;
    match DateTime::from_timestamp(secs, subsec as u32) {
        Some(utc) => {
            let local = utc.with_timezone(&Local);
            local.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        None => format!("{} (invalid)", ns),
    }
}

/// Clamp a selection index to a list length. Returns `idx` if in range, or the last valid index (`len.saturating_sub(1)`), or 0 when `len == 0`.
pub fn clamp_selection(idx: usize, len: usize) -> usize {
    idx.min(len.saturating_sub(1))
}

/// Like [clamp_selection] but returns [None] when `len == 0` so callers can pass through to `select(None)`.
pub fn clamp_selection_opt(idx: usize, len: usize) -> Option<usize> {
    if len == 0 {
        None
    } else {
        Some(clamp_selection(idx, len))
    }
}

/// Pads a string with spaces for block/popup titles, e.g. `" Delta "`.
pub fn frame_string_with_spaces(s: &str) -> String {
    format!(" {} ", s)
}

/// Types that can pad a string for block/popup titles (e.g. `" Delta "`). Shared by [crate::ui::UiStrings] and [crate::utils::UiGlyphs].
pub trait StringObjTraits {
    /// Pads a label with spaces for block/popup titles, e.g. `pad("Delta")` → `" Delta "`.
    fn pad(&self, s: &str) -> String {
        frame_string_with_spaces(s)
    }
}
