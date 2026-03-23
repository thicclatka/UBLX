//! Image viewer: decode, tiered downscale by file size, optional background thread for large files,
//! and `ratatui-image` terminal preview.

use std::sync::mpsc::{self, TryRecvError};

use image::imageops::FilterType;
use ratatui_image::{Resize, StatefulImage, protocol::StatefulProtocol};
use zahirscan::FileType;

use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::ui::UI_GLYPHS;
use crate::utils::{HALF_MIB_BYTES, MIB};

/// Decode + downscale off the UI thread when the file is at least this large (keeps dev/`opt-level=1` snappy too).
/// Same value as [`crate::utils::HALF_MIB_BYTES`] (viewer read cap).
pub const ASYNC_DECODE_MIN_BYTES: u64 = HALF_MIB_BYTES;

#[inline]
pub fn is_image_category(rc: &RightPaneContent) -> bool {
    rc.viewer_zahir_type == Some(FileType::Image)
}

/// Right-pane text under the **Image** heading (e.g. loading line).
#[must_use]
pub fn label_body(raw: &str) -> String {
    format!("{}: {raw}", FileType::Image.as_metadata_name())
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

/// Upper bound on longest edge (px) from the preview **area in terminal cells** so we don’t decode
/// more pixels than can appear in the pane (half-blocks ≈ a few px per cell; this is conservative).
#[must_use]
pub fn max_edge_for_viewport_cells(width_cells: u16, height_cells: u16) -> u32 {
    let w = width_cells as u32;
    let h = height_cells as u32;
    // ~8×16 px per cell is typical; cap longest edge to something drawable in the rect.
    let by_w = w.saturating_mul(8);
    let by_h = h.saturating_mul(16);
    by_w.max(by_h).max(320)
}

/// Longest edge (px) after decode, tiered by **file size** (smaller caps for heavier files = less work in `thumbnail` + terminal encode).
#[must_use]
pub fn tiered_max_dimension_for_file_size(file_size_bytes: u64) -> u32 {
    match file_size_bytes {
        s if s >= 32 * MIB => 768,
        s if s >= 20 * MIB => 1024,
        s if s >= 8 * MIB => 1280,
        s if s >= 2 * MIB => 1600,
        // 1–2 MiB: cap before terminal encode.
        s if s >= MIB => 1440,
        // Under 1 MiB: still bound preview size (terminal encode scales with pixels).
        _ => 1600,
    }
}

#[inline]
pub fn downscale_with_max(img: image::DynamicImage, max_dim: u32) -> image::DynamicImage {
    let w = img.width();
    let h = img.height();
    if w <= max_dim && h <= max_dim {
        img
    } else {
        img.thumbnail(max_dim, max_dim)
    }
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

/// Load [`ViewerImageState::protocol`] when the viewer is an Image row (category **Image**).
///
/// `viewport_cells`: `(width, height)` of the padded preview area in **terminal cells**; pass
/// [`None`] to use only file-size tiers (e.g. tests). When set, decode size is `min(tier, viewport)`.
pub fn ensure_viewer_image(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    viewport_cells: Option<(u16, u16)>,
) {
    if state.right_pane_mode != RightPaneMode::Viewer {
        return;
    }
    if !is_image_category(right_content) {
        state.viewer_image.clear();
        return;
    }
    let Some(abs) = right_content.viewer_abs_path.as_ref() else {
        state.viewer_image.clear();
        state.viewer_image.err = Some("No absolute path for image".to_string());
        return;
    };
    let key = abs.display().to_string();

    // Same file as last tick: poll background decode if any.
    if state.viewer_image.key.as_deref() == Some(key.as_str()) {
        if let Some(rx) = state.viewer_image.decode_rx.as_ref() {
            match rx.try_recv() {
                Ok(Ok(img)) => {
                    state.viewer_image.decode_rx = None;
                    finish_protocol_from_image(state, img);
                }
                Ok(Err(e)) => {
                    state.viewer_image.decode_rx = None;
                    state.viewer_image.err = Some(format!("Could not open image: {e}"));
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    state.viewer_image.decode_rx = None;
                    state.viewer_image.err = Some("Image decode cancelled".to_string());
                }
            }
        }
        return;
    }

    // New selection: cancel in-flight decode; stash previous finished preview; try LRU hit.
    state.viewer_image.decode_rx = None;

    if let Some(pk) = state.viewer_image.key.clone()
        && pk != key
        && let Some(p) = state.viewer_image.protocol.take()
    {
        state.viewer_image.push_lru(pk, p);
    }

    if let Some(p) = state.viewer_image.take_from_lru(&key) {
        state.viewer_image.key = Some(key.clone());
        state.viewer_image.protocol = Some(p);
        state.viewer_image.err = None;
        return;
    }

    state.viewer_image.key = Some(key.clone());
    state.viewer_image.protocol = None;
    state.viewer_image.err = None;

    let file_size = std::fs::metadata(abs).map(|m| m.len()).unwrap_or(0);
    let tiered = tiered_max_dimension_for_file_size(file_size);
    let max_dim = match viewport_cells {
        Some((w, h)) => tiered.min(max_edge_for_viewport_cells(w, h)),
        None => tiered,
    };

    if file_size >= ASYNC_DECODE_MIN_BYTES {
        let (tx, rx) = mpsc::channel();
        state.viewer_image.decode_rx = Some(rx);
        let path = abs.clone();
        std::thread::spawn(move || {
            let res = image::open(&path).map(|img| downscale_with_max(img, max_dim));
            let _ = tx.send(res.map_err(|e| e.to_string()));
        });
    } else {
        match image::open(abs) {
            Ok(img) => finish_protocol_from_image(state, downscale_with_max(img, max_dim)),
            Err(e) => {
                state.viewer_image.err = Some(format!("Could not open image: {e}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::label_body_error;
    use crate::ui::UI_GLYPHS;

    #[test]
    fn label_body_error_includes_markdown_image_glyph() {
        let s = label_body_error("not found");
        assert!(s.contains("not found"));
        assert!(s.contains(UI_GLYPHS.markdown_image));
    }
}
