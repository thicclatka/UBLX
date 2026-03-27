//! Load and fill right-pane content for the current selection (tree, file viewer, zahir JSON).
//! Moved from layout so "get the data that goes into the view" lives with other handlers.

use serde_json::{self, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::db_ops;
use crate::integrations::{ZahirFileType as FileType, file_type_from_metadata_name};
use crate::layout::setup::{
    CATEGORY_DIRECTORY, RightPaneContent, SectionedPreview, TuiRow, UblxState, ViewData,
};
use crate::utils;

/// Run `tree` on `full_path`; use cached result if keyed by `path`. Updates `state.cached_tree`.
fn tree_for_path(state_mut: &mut UblxState, path_ref: &str, full_path_ref: &Path) -> String {
    if let Some((cached_path, text)) = state_mut.cached_tree.as_ref()
        && cached_path == path_ref
    {
        return text.clone();
    }
    {
        match Command::new("tree").arg(full_path_ref).output() {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout).into_owned();
                state_mut.cached_tree = Some((path_ref.to_string(), text.clone()));
                text
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                state_mut.cached_tree = None;
                format!(
                    "tree failed: {}",
                    stderr.trim().lines().next().unwrap_or("unknown")
                )
            }
            Err(e) => {
                state_mut.cached_tree = None;
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
    mtime_ns: Option<i64>,
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
        viewer_mtime_ns: mtime_ns,
        viewer_can_open: false,
        viewer_offer_enhance_zahir: false,
        viewer_offer_enhance_directory_policy: offer_directory_policy,
        viewer_embedded_cover_raster: None,
    }
}

/// Inputs for [`resolve_non_directory_right_pane`]: selected row + resolved path + DB mtime.
struct NonDirectoryPaneInputs<'a> {
    path: &'a str,
    category: &'a str,
    size: u64,
    full_path: PathBuf,
    viewer_mtime_ns: Option<i64>,
    enable_enhance_all: bool,
}

/// File or non-directory row: tree if the path is a directory on disk; otherwise viewer + optional zahir JSON.
fn resolve_non_directory_right_pane(
    state_mut: &mut UblxState,
    db_path_ref: &Path,
    inputs: NonDirectoryPaneInputs<'_>,
) -> RightPaneContent {
    let NonDirectoryPaneInputs {
        path,
        category,
        size,
        full_path,
        viewer_mtime_ns,
        enable_enhance_all,
    } = inputs;
    if full_path.is_dir() {
        let tree_str = tree_for_path(state_mut, path, &full_path);
        return tree_viewer(tree_str, path, full_path, false, viewer_mtime_ns);
    }
    state_mut.cached_tree = None;
    let viewer_zahir_type = file_type_from_metadata_name(category);
    let embedded_cover = match viewer_zahir_type {
        Some(ft @ (FileType::Audio | FileType::Epub)) => utils::try_extract_cover(&full_path, ft),
        _ => None,
    };
    let viewer_str = if embedded_cover.is_some() {
        Some(String::new())
    } else {
        utils::file_content_for_viewer(&full_path, viewer_zahir_type)
    };
    let viewer_byte_size = viewer_str.as_ref().map(|_| size);
    let viewer_can_open = !utils::is_likely_binary(&full_path)
        || matches!(
            viewer_zahir_type,
            Some(
                FileType::Image
                    | FileType::Pdf
                    | FileType::Video
                    | FileType::Audio
                    | FileType::Epub
            )
        );
    let zahir_json: String = db_ops::load_zahir_json_for_path(db_path_ref, path)
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
            viewer_offer_enhance_zahir: !enable_enhance_all,
            viewer_offer_enhance_directory_policy: false,
            viewer_embedded_cover_raster: embedded_cover.clone(),
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
                    viewer_offer_enhance_zahir: false,
                    viewer_offer_enhance_directory_policy: false,
                    viewer_embedded_cover_raster: embedded_cover.clone(),
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
                viewer_offer_enhance_zahir: false,
                viewer_offer_enhance_directory_policy: false,
                viewer_embedded_cover_raster: embedded_cover.clone(),
            },
        }
    }
}

/// Resolve right-pane strings from current selection: directory => tree; file => zahir sections.
/// `zahir_json` is loaded from the DB only for the selected row (lazy load).
/// For snapshot mode pass `Some(all_rows)`; for delta mode pass `None` (view holds rows).
pub fn resolve_right_pane_content(
    state_mut: &mut UblxState,
    dir_to_ublx_ref: &Path,
    db_path_ref: &Path,
    view_ref: &ViewData,
    all_rows_ref: Option<&[TuiRow]>,
    enable_enhance_all: bool,
) -> RightPaneContent {
    let selected: Option<&TuiRow> = state_mut
        .panels
        .content_state
        .selected()
        .and_then(|i| view_ref.row_at(i, all_rows_ref));
    if let Some((path, category, size)) = selected {
        let path: &str = path.as_str();
        let full_path = utils::resolve_under_root(dir_to_ublx_ref, path);
        let viewer_mtime_ns = db_ops::load_mtime_for_path(db_path_ref, path)
            .ok()
            .flatten();
        if *category == CATEGORY_DIRECTORY {
            let tree_str = tree_for_path(state_mut, path, &full_path);
            tree_viewer(tree_str, path, full_path, true, viewer_mtime_ns)
        } else {
            resolve_non_directory_right_pane(
                state_mut,
                db_path_ref,
                NonDirectoryPaneInputs {
                    path,
                    category,
                    size: *size,
                    full_path,
                    viewer_mtime_ns,
                    enable_enhance_all,
                },
            )
        }
    } else {
        state_mut.cached_tree = None;
        RightPaneContent::empty()
    }
}

/// Build `SectionedPreview` (templates, metadata, writing) from zahir JSON value.
#[must_use]
pub fn sectioned_preview_from_zahir(value_ref: &serde_json::Value) -> SectionedPreview {
    let templates = value_ref
        .get("templates")
        .and_then(|t| serde_json::to_string_pretty(t).ok())
        .filter(|s| !s.is_empty() && s != "null" && s != "[]")
        .unwrap_or_default();

    let metadata = value_ref.as_object().and_then(|obj| {
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

    let writing = value_ref
        .get("writing_footprint")
        .and_then(|w| serde_json::to_string_pretty(w).ok());

    SectionedPreview {
        templates,
        metadata,
        writing,
    }
}
