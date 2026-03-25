//! Image / PDF page viewer: decode, tiered downscale by file size, optional background thread
//! for large files or PDF rasterization, and `ratatui-image` terminal preview.

use std::sync::Arc;
use std::sync::atomic::Ordering;
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
#[must_use]
pub fn is_image_category(rc: &RightPaneContent) -> bool {
    rc.viewer_zahir_type == Some(FileType::Image)
}

/// True when the viewer should show a **raster** preview (image file, PDF page, or embedded audio/EPUB cover).
#[inline]
#[must_use]
pub fn is_raster_preview_category(rc: &RightPaneContent) -> bool {
    rc.viewer_embedded_cover_raster
        .as_ref()
        .is_some_and(|b| !b.is_empty())
        || matches!(rc.viewer_zahir_type, Some(FileType::Image | FileType::Pdf))
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
        Some(FileType::Audio) => FileType::Audio.as_metadata_name(),
        Some(FileType::Epub) => FileType::Epub.as_metadata_name(),
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
    let p = viewer.pdf.page.max(1);
    match viewer.pdf.page_count {
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
        if state.viewer_image.pdf.for_path.take().is_some() {
            state
                .viewer_image
                .pdf
                .prefetch_cancel
                .fetch_add(1, Ordering::SeqCst);
            state.viewer_image.pdf.prefetch_rx = None;
            state.viewer_image.pdf.prefetch_earliest = None;
            state.viewer_image.pdf.page = 1;
            state.viewer_image.pdf.page_count = None;
            state.viewer_image.pdf.page_count_rx = None;
        }
        return;
    }
    let Some(abs) = right_content.viewer_abs_path.as_ref() else {
        return;
    };
    if state.viewer_image.pdf.for_path.as_ref() != Some(abs) {
        state
            .viewer_image
            .pdf
            .prefetch_cancel
            .fetch_add(1, Ordering::SeqCst);
        state.viewer_image.pdf.prefetch_rx = None;
        state.viewer_image.pdf.prefetch_earliest =
            Some(std::time::Instant::now() + pdf_preview::PDFPrefetch::DEBOUNCE);
        state.viewer_image.pdf.for_path = Some(abs.clone());
        state.viewer_image.pdf.page = 1;
        state.viewer_image.pdf.page_count = None;
        state.viewer_image.pdf.page_count_rx = None;
        let (tx, rx) = mpsc::channel();
        state.viewer_image.pdf.page_count_rx = Some(rx);
        let p = abs.clone();
        std::thread::spawn(move || {
            let _ = tx.send(pdf_preview::pdf_page_count(&p));
        });
    }
    if let Some(rx) = state.viewer_image.pdf.page_count_rx.as_ref() {
        match rx.try_recv() {
            Ok(Ok(n)) => {
                state.viewer_image.pdf.page_count = Some(n);
                state.viewer_image.pdf.page_count_rx = None;
                state.viewer_image.pdf.page = state.viewer_image.pdf.page.min(n.max(1));
            }
            Ok(Err(_)) | Err(TryRecvError::Disconnected) => {
                state.viewer_image.pdf.page_count_rx = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}

#[inline]
#[must_use]
fn pdf_raster_max_dim(file_size: u64, viewport_cells: Option<(u16, u16)>) -> u32 {
    let tiered = raster_policy::tiered_max_dimension_for_file_size(file_size);
    let max_dim = match viewport_cells {
        Some((w, h)) => {
            let edge = raster_policy::max_edge_for_pdf_viewport_cells(w, h);
            tiered.min(edge)
        }
        None => tiered,
    };
    pdf_preview::PdfRasterMaxDimBoost::apply(max_dim)
}

fn drain_pdf_prefetch_results(state: &mut UblxState, expected_path: &std::path::Path) {
    if state.viewer_image.pdf.for_path.as_deref() != Some(expected_path) {
        return;
    }
    let Some(rx) = state.viewer_image.pdf.prefetch_rx.take() else {
        return;
    };
    loop {
        match rx.try_recv() {
            Ok((key, res)) => {
                if let Ok(img) = res {
                    state.viewer_image.remove_lru_key(&key);
                    let picker = state.viewer_image.picker.get_or_insert_with(|| {
                        ratatui_image::picker::Picker::from_query_stdio()
                            .unwrap_or_else(|_| ratatui_image::picker::Picker::halfblocks())
                    });
                    let proto = picker.new_resize_protocol(img);
                    state.viewer_image.push_lru(key, proto);
                }
            }
            Err(TryRecvError::Empty) => {
                state.viewer_image.pdf.prefetch_rx = Some(rx);
                break;
            }
            Err(TryRecvError::Disconnected) => {
                state.viewer_image.pdf.prefetch_rx = None;
                break;
            }
        }
    }
}

fn maybe_spawn_pdf_prefetch(
    state: &mut UblxState,
    abs: &std::path::Path,
    viewport_cells: Option<(u16, u16)>,
) {
    if state.viewer_image.pdf.prefetch_rx.is_some() {
        return;
    }
    let Some(n) = state.viewer_image.pdf.page_count else {
        return;
    };
    if n <= 1 {
        return;
    }
    let Some(earliest) = state.viewer_image.pdf.prefetch_earliest else {
        return;
    };
    if std::time::Instant::now() < earliest {
        return;
    }
    state.viewer_image.pdf.prefetch_earliest = None;

    let file_size = std::fs::metadata(abs).map(|m| m.len()).unwrap_or(0);
    let max_dim = pdf_raster_max_dim(file_size, viewport_cells);
    let cancel = Arc::clone(&state.viewer_image.pdf.prefetch_cancel);
    let token = cancel.load(Ordering::SeqCst);
    let path = abs.to_path_buf();
    let last_page = (1 + pdf_preview::PDFPrefetch::MAX_EXTRA_PAGES).min(n);

    let (tx, rx) = mpsc::channel();
    state.viewer_image.pdf.prefetch_rx = Some(rx);

    std::thread::spawn(move || {
        for page in 2..=last_page {
            if cancel.load(Ordering::SeqCst) != token {
                return;
            }
            let res = pdf_preview::render_pdf_page(&path, page, max_dim)
                .map(|img| raster_policy::downscale_with_max(img, max_dim));
            let key = format!("{}#p{}", path.display(), page);
            if tx.send((key, res)).is_err() {
                return;
            }
        }
    });
}

#[must_use]
fn raster_max_dimension_for_file_size(
    file_size: u64,
    is_pdf: bool,
    viewport_cells: Option<(u16, u16)>,
) -> u32 {
    if is_pdf {
        pdf_raster_max_dim(file_size, viewport_cells)
    } else {
        let tiered = raster_policy::tiered_max_dimension_for_file_size(file_size);
        match viewport_cells {
            Some((w, h)) => {
                let edge = raster_policy::max_edge_for_viewport_cells(w, h);
                tiered.min(edge)
            }
            None => tiered,
        }
    }
}

/// If the current viewer key matches `selection_key`, poll the background decode channel. Returns `true` when the caller should return (same selection).
fn poll_decode_rx_if_same_selection(state: &mut UblxState, selection_key: &str) -> bool {
    if state.viewer_image.key.as_deref() != Some(selection_key) {
        return false;
    }
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
    true
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

    if right_content.viewer_zahir_type == Some(FileType::Pdf)
        && let Some(abs) = right_content.viewer_abs_path.as_ref()
    {
        drain_pdf_prefetch_results(state, abs);
        maybe_spawn_pdf_prefetch(state, abs, viewport_cells);
    }

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
    let has_embedded_cover = right_content
        .viewer_embedded_cover_raster
        .as_ref()
        .is_some_and(|b| !b.is_empty());
    let path_str = abs.display().to_string();
    let selection_key = if is_pdf {
        format!("{}#p{}", path_str, state.viewer_image.pdf.page.max(1))
    } else if has_embedded_cover {
        format!("{path_str}#embedded-cover")
    } else {
        path_str
    };

    // Same file + page: poll background decode if any.
    if poll_decode_rx_if_same_selection(state, selection_key.as_str()) {
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
    let max_dim = raster_max_dimension_for_file_size(file_size, is_pdf, viewport_cells);

    if is_pdf {
        let (tx, rx) = mpsc::channel();
        state.viewer_image.decode_rx = Some(rx);
        let path = abs.clone();
        let page = state.viewer_image.pdf.page.max(1);
        std::thread::spawn(move || {
            let res = pdf_preview::render_pdf_page(&path, page, max_dim)
                .map(|img| raster_policy::downscale_with_max(img, max_dim));
            let _ = tx.send(res);
        });
    } else if let Some(bytes) = right_content
        .viewer_embedded_cover_raster
        .as_ref()
        .filter(|b| !b.is_empty())
    {
        if bytes.len() as u64 >= ASYNC_DECODE_MIN_BYTES {
            let (tx, rx) = mpsc::channel();
            state.viewer_image.decode_rx = Some(rx);
            let bytes = bytes.to_vec();
            std::thread::spawn(move || {
                let res = image::load_from_memory(&bytes)
                    .map(|img| raster_policy::downscale_with_max(img, max_dim))
                    .map_err(|e| e.to_string());
                let _ = tx.send(res);
            });
        } else {
            match image::load_from_memory(bytes)
                .map(|img| raster_policy::downscale_with_max(img, max_dim))
            {
                Ok(img) => finish_protocol_from_image(state, img),
                Err(e) => {
                    state.viewer_image.err = Some(format!("Could not decode cover: {e}"));
                }
            }
        }
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
                finish_protocol_from_image(state, raster_policy::downscale_with_max(img, max_dim));
            }
            Err(e) => {
                state.viewer_image.err = Some(format!("Could not open image: {e}"));
            }
        }
    }
}
