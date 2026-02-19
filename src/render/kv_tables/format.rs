//! Key/value and value display formatting for metadata and contents tables.

use crate::utils::format_bytes;

/// Words that are displayed in full caps (e.g. "svo" -> "SVO", "pdf" -> "PDF").
const ALL_CAPS: &[&str] = &[
    "svo", "pdf", "tiff", "jpeg", "png", "rgb", "yuv", "aac", "mp3", "h264", "cbr", "lf", "und",
    "eng",
];

pub fn format_key(key: &str) -> String {
    let words: Vec<String> = key
        .split('_')
        .map(|w| {
            let lower = w.to_lowercase();
            if ALL_CAPS.iter().any(|&c| c == lower) {
                w.to_uppercase()
            } else {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(c).collect(),
                }
            }
        })
        .collect();
    words.join(" ")
}

pub fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "—".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else if arr.len() <= 3 && arr.iter().all(|x| x.is_string() || x.is_number()) {
                arr.iter()
                    .map(value_to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                format!("[{} items]", arr.len())
            }
        }
        serde_json::Value::Object(_) => "{…}".to_string(),
    }
}

/// Format value for display: byte format when key contains "size", "compressed", or "uncompressed" (and value is numeric); "%" when key contains "percent" (case-insensitive).
pub fn format_value(v: &serde_json::Value, key: &str) -> String {
    let key_lower = key.to_lowercase();
    let is_byte_key = key_lower.contains("size")
        || key_lower.contains("compressed")
        || key_lower.contains("uncompressed");
    if is_byte_key {
        if let Some(n) = v.as_u64() {
            return format_bytes(n);
        }
        if let Some(n) = v.as_i64().filter(|&x| x >= 0) {
            return format_bytes(n as u64);
        }
    }
    let s = value_to_string(v);
    if key_lower.contains("percent") {
        format!("{s}%")
    } else {
        s
    }
}
