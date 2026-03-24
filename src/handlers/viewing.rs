//! Load and fill right-pane content for the current selection (tree, file viewer, zahir JSON).
//! Moved from layout so "get the data that goes into the view" lives with other handlers.

use serde_json::{self, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::db_ops;
use crate::handlers::zahir_ops::{ZahirFileType as FileType, file_type_from_metadata_name};
use crate::layout::setup::{
    CATEGORY_DIRECTORY, RightPaneContent, SectionedPreview, TuiRow, UblxState, ViewData,
};
use crate::utils::{file_content_for_viewer, is_likely_binary, path::resolve_under_root};

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

fn tree_viewer(
    tree_str: String,
    rel_path: &str,
    abs_path: PathBuf,
    offer_directory_policy: bool,
) -> RightPaneContent {
    RightPaneContent {
        templates: String::new(),
        metadata: None,
        writing: None,
        viewer: Some(tree_str),
        viewer_path: Some(rel_path.to_string()),
        viewer_abs_path: Some(abs_path),
        viewer_zahir_type: None,
        viewer_byte_size: None,
        viewer_mtime_ns: None,
        viewer_can_open: false,
        open_hint_label: None,
        viewer_offer_enhance_zahir: false,
        viewer_offer_enhance_directory_policy: offer_directory_policy,
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
    enable_enhance_all: bool,
) -> RightPaneContent {
    let selected: Option<&TuiRow> = state
        .panels
        .content_state
        .selected()
        .and_then(|i| view.row_at(i, all_rows));
    if let Some((path, category, size)) = selected {
        let path: &str = path.as_str();
        let full_path = resolve_under_root(dir_to_ublx, path);
        if *category == CATEGORY_DIRECTORY {
            let tree_str = tree_for_path(state, path, &full_path);
            tree_viewer(tree_str, path, full_path, true)
        } else {
            if full_path.is_dir() {
                let tree_str = tree_for_path(state, path, &full_path);
                return tree_viewer(tree_str, path, full_path, false);
            }
            state.cached_tree = None;
            let viewer_zahir_type = file_type_from_metadata_name(category);
            let viewer_str = file_content_for_viewer(&full_path, viewer_zahir_type);
            let viewer_byte_size = viewer_str.as_ref().map(|_| *size);
            let viewer_mtime_ns = db_ops::load_mtime_for_path(db_path, path).ok().flatten();
            let viewer_can_open = !is_likely_binary(&full_path)
                || matches!(viewer_zahir_type, Some(FileType::Image | FileType::Pdf));
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
                    viewer_abs_path: Some(full_path.clone()),
                    viewer_zahir_type,
                    viewer_byte_size,
                    viewer_mtime_ns,
                    viewer_can_open,
                    open_hint_label: None,
                    viewer_offer_enhance_zahir: !enable_enhance_all,
                    viewer_offer_enhance_directory_policy: false,
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
                            viewer_abs_path: Some(full_path.clone()),
                            viewer_zahir_type,
                            viewer_byte_size,
                            viewer_mtime_ns,
                            viewer_can_open,
                            open_hint_label: None,
                            viewer_offer_enhance_zahir: false,
                            viewer_offer_enhance_directory_policy: false,
                        }
                    }
                    _ => RightPaneContent {
                        templates: String::new(),
                        metadata: None,
                        writing: None,
                        viewer: viewer_str,
                        viewer_path: Some(path.to_string()),
                        viewer_abs_path: Some(full_path.clone()),
                        viewer_zahir_type,
                        viewer_byte_size,
                        viewer_mtime_ns,
                        viewer_can_open,
                        open_hint_label: None,
                        viewer_offer_enhance_zahir: false,
                        viewer_offer_enhance_directory_policy: false,
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
