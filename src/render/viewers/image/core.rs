//! Image / PDF page viewer: decode, tiered downscale by file size, optional background thread
//! for large files or PDF rasterization, and `ratatui-image` terminal preview.

use std::sync::mpsc::{self, TryRecvError};

use image::imageops::FilterType;
use ratatui_image::{Resize, StatefulImage, protocol::StatefulProtocol};

use super::raster_policy;
use crate::render::viewers::pdf_preview;

use crate::handlers::zahir_ops::ZahirFileType as FileType;
use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState, ViewerImageState};
use crate::ui::UI_GLYPHS;
use crate::utils::HALF_MIB_BYTES;

/// Decode + downscale off the UI thread when the file is at least this large (keeps dev/`opt-level=1` snappy too).
/// Same value as [`crate::utils::HALF_MIB_BYTES`] (viewer read cap).
pub const ASYNC_DECODE_MIN_BYTES: u64 = HALF_MIB_BYTES;

#[inline]
pub fn is_image_category(rc: &RightPaneContent) -> bool {
    rc.viewer_zahir_type == Some(FileType::Image)
}

/// True when the viewer should show a **raster** preview (standalone image file or PDF page).
#[inline]
pub fn is_raster_preview_category(rc: &RightPaneContent) -> bool {
    matches!(rc.viewer_zahir_type, Some(FileType::Image | FileType::Pdf))
}

/// Right-pane text under the **Image** heading (e.g. loading line).
#[must_use]
pub fn label_body(raw: &str) -> String {
    format!("{}: {raw}", FileType::Image.as_metadata_name())
}

/// Label for image or PDF raster preview (uses snapshot category name).
#[must_use]
pub fn raster_preview_label_body(rc: &RightPaneContent, raw: &str) -> String {
    let name = match rc.viewer_zahir_type {
        Some(FileType::Pdf) => FileType::Pdf.as_metadata_name(),
        _ => FileType::Image.as_metadata_name(),
    };
    format!("{name}: {raw}")
}

/// Footer line for PDF page position (`Page 2 / 10`).
#[must_use]
pub fn pdf_page_footer_text(right: &RightPaneContent, viewer: &ViewerImageState) -> Option<String> {
    if right.viewer_zahir_type != Some(FileType::Pdf) || right.viewer_abs_path.is_none() {
        return None;
    }
    let p = viewer.pdf_page.max(1);
    match viewer.pdf_page_count {
        Some(n) => Some(format!("Page {p} / {n}")),
        None => Some(format!("Page {p}")),
    }
}

/// Like [`label_body`], but prefixes the body with the markdown image glyph — **only** for failed
/// `![](...)` previews in markdown, not for standalone **Image** category files ([`label_body`]).
#[must_use]
pub fn label_body_error(raw: &str) -> String {
    format!(
        "{}: {}{}",
        FileType::Image.as_metadata_name(),
        UI_GLYPHS.markdown_image,
        raw
    )
}

fn finish_protocol_from_image(state: &mut UblxState, dyn_img: image::DynamicImage) {
    let picker = state.viewer_image.picker.get_or_insert_with(|| {
        ratatui_image::picker::Picker::from_query_stdio()
            .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks())
    });
    let proto = picker.new_resize_protocol(dyn_img);
    state.viewer_image.protocol = Some(proto);
}

/// [`StatefulImage`] with fast nearest-neighbor fitting.
#[inline]
#[must_use]
pub fn stateful_widget() -> StatefulImage<StatefulProtocol> {
    StatefulImage::<StatefulProtocol>::default().resize(Resize::Fit(Some(FilterType::Nearest)))
}

/// Reset PDF page state when switching files; poll [`ViewerImageState::pdf_page_count_rx`].
pub fn sync_pdf_selection_state(state: &mut UblxState, right_content: &RightPaneContent) {
    if right_content.viewer_zahir_type != Some(FileType::Pdf) {
        if state.viewer_image.pdf_for_path.take().is_some() {
            state.viewer_image.pdf_page = 1;
            state.viewer_image.pdf_page_count = None;
            state.viewer_image.pdf_page_count_rx = None;
        }
        return;
    }
    let Some(abs) = right_content.viewer_abs_path.as_ref() else {
        return;
    };
    if state.viewer_image.pdf_for_path.as_ref() != Some(abs) {
        state.viewer_image.pdf_for_path = Some(abs.clone());
        state.viewer_image.pdf_page = 1;
        state.viewer_image.pdf_page_count = None;
        state.viewer_image.pdf_page_count_rx = None;
        let (tx, rx) = mpsc::channel();
        state.viewer_image.pdf_page_count_rx = Some(rx);
        let p = abs.clone();
        std::thread::spawn(move || {
            let _ = tx.send(pdf_preview::pdf_page_count(&p));
        });
    }
    if let Some(rx) = state.viewer_image.pdf_page_count_rx.as_ref() {
        match rx.try_recv() {
            Ok(Ok(n)) => {
                state.viewer_image.pdf_page_count = Some(n);
                state.viewer_image.pdf_page_count_rx = None;
                state.viewer_image.pdf_page = state.viewer_image.pdf_page.min(n.max(1));
            }
            Ok(Err(_)) => {
                state.viewer_image.pdf_page_count_rx = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                state.viewer_image.pdf_page_count_rx = None;
            }
        }
    }
}

/// Load [`ViewerImageState::protocol`] when the viewer is an **Image** or **PDF** row (raster preview).
///
/// `viewport_cells`: `(width, height)` of the padded preview area in **terminal cells**; pass
/// [`None`] to use only file-size tiers (e.g. tests). When set, decode size is `min(tier, viewport)`.
pub fn ensure_viewer_image(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    viewport_cells: Option<(u16, u16)>,
) {
    sync_pdf_selection_state(state, right_content);

    if state.right_pane_mode != RightPaneMode::Viewer {
        return;
    }
    if !is_raster_preview_category(right_content) {
        state.viewer_image.clear();
        return;
    }
    let Some(abs) = right_content.viewer_abs_path.as_ref() else {
        state.viewer_image.clear();
        state.viewer_image.err = Some("No absolute path for preview".to_string());
        return;
    };

    let is_pdf = right_content.viewer_zahir_type == Some(FileType::Pdf);
    let path_str = abs.display().to_string();
    let selection_key = if is_pdf {
        format!("{}#p{}", path_str, state.viewer_image.pdf_page.max(1))
    } else {
        path_str
    };

    // Same file + page: poll background decode if any.
    if state.viewer_image.key.as_deref() == Some(selection_key.as_str()) {
        if let Some(rx) = state.viewer_image.decode_rx.as_ref() {
            match rx.try_recv() {
                Ok(Ok(img)) => {
                    state.viewer_image.decode_rx = None;
                    finish_protocol_from_image(state, img);
                }
                Ok(Err(e)) => {
                    state.viewer_image.decode_rx = None;
                    state.viewer_image.err = Some(format!("Could not load preview: {e}"));
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    state.viewer_image.decode_rx = None;
                    state.viewer_image.err = Some("Preview decode cancelled".to_string());
                }
            }
        }
        return;
    }

    // New selection or new PDF page: cancel in-flight decode; stash previous finished preview; try LRU hit.
    state.viewer_image.decode_rx = None;

    if let Some(pk) = state.viewer_image.key.clone()
        && pk != selection_key
        && let Some(p) = state.viewer_image.protocol.take()
    {
        state.viewer_image.push_lru(pk, p);
    }

    if let Some(p) = state.viewer_image.take_from_lru(&selection_key) {
        state.viewer_image.key = Some(selection_key.clone());
        state.viewer_image.protocol = Some(p);
        state.viewer_image.err = None;
        return;
    }

    state.viewer_image.key = Some(selection_key.clone());
    state.viewer_image.protocol = None;
    state.viewer_image.err = None;

    let file_size = std::fs::metadata(abs).map(|m| m.len()).unwrap_or(0);
    let tiered = raster_policy::tiered_max_dimension_for_file_size(file_size);
    let max_dim = match viewport_cells {
        Some((w, h)) => {
            let edge = if is_pdf {
                raster_policy::max_edge_for_pdf_viewport_cells(w, h)
            } else {
                raster_policy::max_edge_for_viewport_cells(w, h)
            };
            tiered.min(edge)
        }
        None => tiered,
    };
    let max_dim = if is_pdf {
        pdf_preview::PdfRasterMaxDimBoost::apply(max_dim)
    } else {
        max_dim
    };

    if is_pdf {
        let (tx, rx) = mpsc::channel();
        state.viewer_image.decode_rx = Some(rx);
        let path = abs.clone();
        let page = state.viewer_image.pdf_page.max(1);
        std::thread::spawn(move || {
            let res = pdf_preview::render_pdf_page(&path, page, max_dim)
                .map(|img| raster_policy::downscale_with_max(img, max_dim));
            let _ = tx.send(res);
        });
    } else if file_size >= ASYNC_DECODE_MIN_BYTES {
        let (tx, rx) = mpsc::channel();
        state.viewer_image.decode_rx = Some(rx);
        let path = abs.clone();
        std::thread::spawn(move || {
            let res = image::open(&path).map(|img| raster_policy::downscale_with_max(img, max_dim));
            let _ = tx.send(res.map_err(|e| e.to_string()));
        });
    } else {
        match image::open(abs) {
            Ok(img) => {
                finish_protocol_from_image(state, raster_policy::downscale_with_max(img, max_dim))
            }
            Err(e) => {
                state.viewer_image.err = Some(format!("Could not open image: {e}"));
            }
        }
    }
}
