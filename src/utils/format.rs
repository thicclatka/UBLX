//! Shared formatting helpers (e.g. timestamps for UI).

use chrono::{DateTime, Local};

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
