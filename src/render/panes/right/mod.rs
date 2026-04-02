//! Right pane: tabs and body for Viewer / Templates / Metadata / Writing (fullscreen included).
//!
//! Submodules: [`content`] (text, tables, cache, line counts), [`chrome`] (frame, tabs, footers),
//! [`draw`] (layout and paint).
//!
//! ## Viewer tab
//!
//! 1. **Width** — [`scrollable_content::area_above_bottom_pad`], then up to six passes: estimate lines at
//!    `text_w` → [`scrollable_content::viewport_text_width`] may drop one column for a scrollbar → repeat
//!    until stable. Call [`content::ensure_viewer_text_cache`] once afterward (async poll + schedule only
//!    there, not while `text_w` is still moving). All layout uses that final `text_w`.
//! 2. **Cache** — [`content::ensure_viewer_text_cache`] populates viewer/CSV caches for heavy paths; byte
//!    thresholds live on [`crate::engine::cache::VIEWER_TEXT_CACHE`]. Painting goes through
//!    [`content::content_display_text`].
//! 3. **Line count** — [`content::viewer_total_lines`] must match painted content. From [`draw`], pass
//!    `scroll_viewport_h: Some(padded.height)` so pending async work does not estimate “one line” and
//!    widen `text_w` only to have a scrollbar after load (that mismatch used to invalidate cache keys and
//!    flicker). Same function branches: metadata/writing JSON → [`kv_tables::content_height`]; CSV → cache
//!    or [`csv_handler::table_line_count`]; large markdown → cache line count; else wrapped/plain. Tables:
//!    Zahir CSV (incl. tsv/tab/psv) or path ending in `.csv`/`.tsv`/`.tab`/`.psv`.
//! 4. **Scroll** — [`scrollable_content::layout_scrollable_content`], [`scrollable_content::draw_scrollbar`].
//! 5. **Paint** — JSON side tabs → [`kv_tables::draw_tables`]; otherwise [`content::content_display_text`]
//!    in a `Paragraph` with optional ratatui `Wrap`.
//! 6. **Raster previews** — [`crate::render::viewers::image::core`]: large blobs decode off-thread,
//!    tiered downscale, small LRU for quick revisit.
//!
//! **Preformatted vs wrap:** [`content::ratatui_wrap_right_paragraph`] is false when Markdown or a
//! delimiter table is fully laid out (no second wrap). Plain text and failed/empty CSV still use `Wrap`.
//!
//! ## Extending
//!
//! Keep display, [`content::viewer_total_lines`], and wrap behavior in sync. Add a cache in state only
//! when reusing by `(path, width)` like CSV.

mod chrome;
mod content;
mod draw;

pub use chrome::visible_tabs;
pub use content::*;
pub use draw::*;
