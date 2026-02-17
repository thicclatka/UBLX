use serde_json;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use super::setup::{CATEGORY_DIRECTORY, RightPaneContent, SectionedPreview, TuiRow, UblxState};

/// Max bytes to load into the viewer for a single file (avoid OOM). Larger files are truncated.
const VIEWER_MAX_BYTES: usize = 512 * 1024;
/// Chunk size for binary check (read this many bytes to detect NUL / invalid UTF-8).
const BINARY_CHECK_CHUNK: usize = 8192;

/// Quick is_binary check: read first chunk, treat as binary if NUL present or invalid UTF-8.
fn is_likely_binary(path: &Path) -> bool {
    let mut f = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = [0u8; BINARY_CHECK_CHUNK];
    let n = f.read(&mut buf).unwrap_or(0);
    let buf = &buf[..n];
    buf.contains(&0) || std::str::from_utf8(buf).is_err()
}

/// Resolve viewer string for a file: (directory), (binary file), (file not found), or file contents (with size limit).
/// Future: per-filetype customization can be added here.
fn file_content_for_viewer(path: &Path) -> Option<String> {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Some("(file not found)".to_string()),
    };
    if meta.is_dir() {
        return Some("(directory)".to_string());
    }
    if meta.is_file() && is_likely_binary(path) {
        return Some("(binary file)".to_string());
    }
    let f = fs::File::open(path).ok()?;
    let cap = VIEWER_MAX_BYTES.min(meta.len() as usize);
    let mut buf = Vec::with_capacity(cap);
    let n = f.take(VIEWER_MAX_BYTES as u64).read_to_end(&mut buf).ok()?;
    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
    let out = if (n as u64) >= meta.len() {
        s
    } else {
        format!("{}\n\n… (truncated, {} bytes total)", s, meta.len())
    };
    Some(out)
}

/// Resolve right-pane strings from current selection: directory => tree; file => zahir sections.
pub fn resolve_right_pane_content(
    state: &mut UblxState,
    dir_to_ublx: &Path,
    filtered_contents_rows: &[TuiRow],
) -> RightPaneContent {
    let selected = state
        .content_state
        .selected()
        .and_then(|i| filtered_contents_rows.get(i));
    match selected {
        Some((path, category, zahir_json, size)) => {
            if *category == CATEGORY_DIRECTORY {
                let tree_str = {
                    let use_cache = state
                        .cached_tree
                        .as_ref()
                        .is_some_and(|(cached_path, _)| cached_path == path);
                    if use_cache {
                        state.cached_tree.as_ref().unwrap().1.clone()
                    } else {
                        let full_path = dir_to_ublx.join(Path::new(path.as_str()));
                        match Command::new("tree").arg(&full_path).output() {
                            Ok(out) if out.status.success() => {
                                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                                state.cached_tree = Some((path.clone(), text.clone()));
                                text
                            }
                            Ok(out) => {
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                state.cached_tree = None;
                                format!(
                                    "tree failed: {}",
                                    stderr.trim().lines().next().unwrap_or("unknown")
                                )
                            }
                            Err(e) => {
                                state.cached_tree = None;
                                format!("tree not available: {}", e)
                            }
                        }
                    }
                };
                RightPaneContent {
                    templates: "no template found".to_string(),
                    metadata: None,
                    writing: None,
                    viewer: Some(tree_str),
                    viewer_path: None,
                    viewer_byte_size: None,
                }
            } else {
                state.cached_tree = None;
                let full_path = dir_to_ublx.join(Path::new(path.as_str()));
                let viewer_str = file_content_for_viewer(&full_path);
                let viewer_byte_size = viewer_str.as_ref().map(|_| *size);
                if zahir_json.is_empty() {
                    RightPaneContent {
                        templates: "no template found".to_string(),
                        metadata: None,
                        writing: None,
                        viewer: viewer_str,
                        viewer_path: Some(path.clone()),
                        viewer_byte_size,
                    }
                } else {
                    match serde_json::from_str::<serde_json::Value>(zahir_json) {
                        Ok(v) => {
                            let s = sectioned_preview_from_zahir(&v);
                            RightPaneContent {
                                templates: s.templates,
                                metadata: s.metadata,
                                writing: s.writing,
                                viewer: viewer_str,
                                viewer_path: Some(path.clone()),
                                viewer_byte_size,
                            }
                        }
                        _ => RightPaneContent {
                            templates: "no template found".to_string(),
                            metadata: None,
                            writing: None,
                            viewer: viewer_str,
                            viewer_path: Some(path.clone()),
                            viewer_byte_size,
                        },
                    }
                }
            }
        }
        None => {
            state.cached_tree = None;
            RightPaneContent {
                templates: "(select an item)".to_string(),
                metadata: None,
                writing: None,
                viewer: None,
                viewer_path: None,
                viewer_byte_size: None,
            }
        }
    }
}

pub fn sectioned_preview_from_zahir(value: &serde_json::Value) -> SectionedPreview {
    let templates = value
        .get("templates")
        .and_then(|t| serde_json::to_string_pretty(t).ok())
        .filter(|s| !s.is_empty() && s != "null" && s != "[]")
        .unwrap_or_else(|| "no template found".to_string());

    let metadata = value.as_object().and_then(|obj| {
        let parts: Vec<String> = obj
            .iter()
            .filter(|(k, _)| k.ends_with("_metadata"))
            .filter_map(|(_, v)| serde_json::to_string_pretty(v).ok())
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    });

    let writing = value
        .get("writing_footprint")
        .and_then(|w| serde_json::to_string_pretty(w).ok());

    SectionedPreview {
        templates,
        metadata,
        writing,
    }
}
