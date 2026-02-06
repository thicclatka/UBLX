use serde_json;
use std::path::Path;
use std::process::Command;

use super::setup::{CATEGORY_DIRECTORY, RightPaneContent, SectionedPreview, TuiRow, UblxState};

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
        Some((path, category, zahir_json)) => {
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
                }
            } else if zahir_json.is_empty() {
                state.cached_tree = None;
                RightPaneContent {
                    templates: "no template found".to_string(),
                    metadata: None,
                    writing: None,
                    viewer: None,
                }
            } else {
                state.cached_tree = None;
                match serde_json::from_str::<serde_json::Value>(zahir_json) {
                    Ok(v) => {
                        let s = sectioned_preview_from_zahir(&v);
                        RightPaneContent {
                            templates: s.templates,
                            metadata: s.metadata,
                            writing: s.writing,
                            viewer: None,
                        }
                    }
                    _ => RightPaneContent {
                        templates: "no template found".to_string(),
                        metadata: None,
                        writing: None,
                        viewer: None,
                    },
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
