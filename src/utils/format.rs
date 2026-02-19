//! Shared formatting helpers (e.g. timestamps for UI).

use chrono::{DateTime, Local};

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
