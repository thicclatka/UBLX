//! Load and fill right-pane content for the current selection (tree, file viewer, zahir JSON).
//! Moved from layout so "get the data that goes into the view" lives with other handlers.

use serde_json::{self, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::db_ops;
use crate::layout::setup::{
    CATEGORY_DIRECTORY, RightPaneContent, SectionedPreview, TuiRow, UblxState, ViewData,
};

/// Max bytes to load into the viewer for a single file (avoid OOM). Larger files are truncated.
const VIEWER_MAX_BYTES: usize = 512 * 1024;

/// [`std::fs::Metadata::len`] is `u64`; saturates at `usize::MAX` on 32-bit. Safe for `.min(small_cap)`:
/// the cap (e.g. [`VIEWER_MAX_BYTES`]) still bounds allocation.
#[inline]
fn u64_to_usize_saturating(len: u64) -> usize {
    usize::try_from(len).unwrap_or(usize::MAX)
}
/// Chunk size for binary check (read this many bytes to detect NUL / invalid UTF-8).
const BINARY_CHECK_CHUNK: usize = 8192;

/// Quick `is_binary` check: read first chunk, treat as binary if NUL present or invalid UTF-8.
fn is_likely_binary(path: &Path) -> bool {
    let Ok(mut f) = fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; BINARY_CHECK_CHUNK];
    let n = f.read(&mut buf).unwrap_or(0);
    let buf = &buf[..n];
    buf.contains(&0) || std::str::from_utf8(buf).is_err()
}

/// Label for a binary file: "EXT file" if path has extension (e.g. "PNG file"), else "binary file".
fn binary_file_label(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).map_or_else(
        || "binary file".to_string(),
        |ext| format!("{} file", ext.to_uppercase()),
    )
}

/// Resolve viewer string for a file: (directory), binary label, (file not found), or file contents (with size limit).
fn file_content_for_viewer(path: &Path) -> Option<String> {
    let Ok(meta) = fs::metadata(path) else {
        return Some("(file not found)".to_string());
    };
    // if meta.is_dir() {
    //     return Some("(directory)".to_string());
    // }
    if meta.is_file() && is_likely_binary(path) {
        return Some(binary_file_label(path));
    }
    let f = fs::File::open(path).ok()?;
    let cap = VIEWER_MAX_BYTES.min(u64_to_usize_saturating(meta.len()));
    let mut buf = Vec::with_capacity(cap);
    let take_limit = u64::try_from(VIEWER_MAX_BYTES).expect("512 KiB fits in u64");
    let n = f.take(take_limit).read_to_end(&mut buf).ok()?;
    let s = String::from_utf8_lossy(&buf[..n]).into_owned();
    // `n` is at most VIEWER_MAX_BYTES, so it always fits in `u64`.
    let n_u64 = u64::try_from(n).expect("bytes read fits in u64");
    let out = if n_u64 >= meta.len() {
        s
    } else {
        format!("{}\n\n… (truncated, {} bytes total)", s, meta.len())
    };
    Some(out)
}

/// Run `tree` on `full_path`; use cached result if keyed by `path`. Updates `state.cached_tree`.
fn tree_for_path(state: &mut UblxState, path: &str, full_path: &Path) -> String {
    if let Some((cached_path, text)) = state.cached_tree.as_ref()
        && cached_path == path
    {
        return text.clone();
    }
    {
        match Command::new("tree").arg(full_path).output() {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                state.cached_tree = Some((path.to_string(), text.clone()));
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
                format!("tree not available: {e}")
            }
        }
    }
}

fn tree_content(tree_str: String) -> RightPaneContent {
    RightPaneContent {
        templates: String::new(),
        metadata: None,
        writing: None,
        viewer: Some(tree_str),
        viewer_path: None,
        viewer_byte_size: None,
        viewer_mtime_ns: None,
        viewer_can_open: false,
        open_hint_label: None,
    }
}

/// Resolve right-pane strings from current selection: directory => tree; file => zahir sections.
/// `zahir_json` is loaded from the DB only for the selected row (lazy load).
/// For snapshot mode pass `Some(all_rows)`; for delta mode pass `None` (view holds rows).
pub fn resolve_right_pane_content(
    state: &mut UblxState,
    dir_to_ublx: &Path,
    db_path: &Path,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
) -> RightPaneContent {
    let selected: Option<&TuiRow> = state
        .panels
        .content_state
        .selected()
        .and_then(|i| view.row_at(i, all_rows));
    if let Some((path, category, size)) = selected {
        let path: &str = path.as_str();
        if *category == CATEGORY_DIRECTORY {
            let full_path = dir_to_ublx.join(Path::new(path));
            let tree_str = tree_for_path(state, path, &full_path);
            tree_content(tree_str)
        } else {
            let full_path: PathBuf = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                dir_to_ublx.join(Path::new(path))
            };
            if full_path.is_dir() {
                let tree_str = tree_for_path(state, path, &full_path);
                return tree_content(tree_str);
            }
            state.cached_tree = None;
            let viewer_str = file_content_for_viewer(&full_path);
            let viewer_byte_size = viewer_str.as_ref().map(|_| *size);
            let viewer_mtime_ns = db_ops::load_mtime_for_path(db_path, path).ok().flatten();
            let viewer_can_open = !is_likely_binary(&full_path);
            let zahir_json: String = db_ops::load_zahir_json_for_path(db_path, path)
                .ok()
                .flatten()
                .unwrap_or_default();
            if zahir_json.is_empty() {
                RightPaneContent {
                    templates: String::new(),
                    metadata: None,
                    writing: None,
                    viewer: viewer_str,
                    viewer_path: Some(path.to_string()),
                    viewer_byte_size,
                    viewer_mtime_ns,
                    viewer_can_open,
                    open_hint_label: None,
                }
            } else {
                match serde_json::from_str::<Value>(&zahir_json) {
                    Ok(v) => {
                        let s = sectioned_preview_from_zahir(&v);
                        RightPaneContent {
                            templates: s.templates,
                            metadata: s.metadata,
                            writing: s.writing,
                            viewer: viewer_str,
                            viewer_path: Some(path.to_string()),
                            viewer_byte_size,
                            viewer_mtime_ns,
                            viewer_can_open,
                            open_hint_label: None,
                        }
                    }
                    _ => RightPaneContent {
                        templates: String::new(),
                        metadata: None,
                        writing: None,
                        viewer: viewer_str,
                        viewer_path: Some(path.to_string()),
                        viewer_byte_size,
                        viewer_mtime_ns,
                        viewer_can_open,
                        open_hint_label: None,
                    },
                }
            }
        }
    } else {
        state.cached_tree = None;
        RightPaneContent::empty()
    }
}

/// Build `SectionedPreview` (templates, metadata, writing) from zahir JSON value.
#[must_use]
pub fn sectioned_preview_from_zahir(value: &serde_json::Value) -> SectionedPreview {
    let templates = value
        .get("templates")
        .and_then(|t| serde_json::to_string_pretty(t).ok())
        .filter(|s| !s.is_empty() && s != "null" && s != "[]")
        .unwrap_or_default();

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
