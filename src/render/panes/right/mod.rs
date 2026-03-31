//! Right pane: tabs, Viewer / Templates / Metadata / Writing content, fullscreen.
//!
//! Submodules: [`content`] (scrollable text, tables, raster labels, cache, line counts), [`chrome`]
//! (frame, tabs, footers), [`draw`] (layout and painting).
//!
//! ## Content pipeline (Viewer tab — width, wrap, cache)
//!
//! 1. **Text width** — After vertical padding, [`scrollable_content::area_above_bottom_pad`], we
//!    iterate (up to 6 times): estimate total lines at width `text_w` →
//!    [`scrollable_content::viewport_text_width`] may shrink width to reserve space for a vertical
//!    scrollbar → repeat until stable. All layout and line counting for the scrollable body must use this
//!    final `text_w`.
//! 2. **Text cache** — [`content::ensure_viewer_text_cache`] fills [`UblxState::viewer_text_cache`]
//!    for delimiter tables (always) and **large** markdown (see
//!    [`crate::render::cache::VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES`]). Scrolling uses a
//!    **viewport slice** into cached [`Text`] (see [`content::content_display_text`]).
//! 3. **Total lines** — [`content::viewer_total_lines`] must match what is actually drawn (scrollbar height).
//!    Branches: JSON metadata/writing → [`kv_tables::content_height`]; CSV → cache or
//!    [`csv_handler::table_line_count`]; large markdown → cache line count; otherwise plain text → wrapped line count.
//!    Delimited table files: delimiter detection in [`content`] — zahir category **CSV** (incl. tsv/tab/psv)
//!    **or** path extension `.csv`/`.tsv`/`.tab`/`.psv` so rows still get tables if metadata category is off.
//! 4. **Scroll** — [`scrollable_content::layout_scrollable_content`] + [`scrollable_content::draw_scrollbar`].
//! 5. **Draw** — If metadata/writing JSON is present, [`kv_tables::draw_tables`] (no `Paragraph`).
//!    Else [`content::content_display_text`] → `Paragraph` with optional ratatui [`Wrap`].
//! 6. **Images** — See [`crate::render::viewers::image::core`]: ≥512 KiB off-thread decode, tiered
//!    downscale, **min(tier, viewport cells)**, and a small **LRU** for instant back within a few files.
//!
//! **Preformatted layout:** [`content::ratatui_wrap_right_paragraph`] reflects when Markdown and successful
//! delimiter tables are fully laid out to the viewport; ratatui must *not* wrap again. For delimiter tables, when
//! the text cache has warmed, the wrap check uses it instead of re-parsing.
//! Plain text and failed/empty CSV still use `Wrap`.
//!
//! ## Adding another content type (new file kind or tab)
//!
//! Keep display, line count, and wrap flag aligned: extend [`content`] and
//! [`content::viewer_total_lines`], and set whether preformatted layout should skip `Wrap`.
//! Add a cache in state only if you need `(path, width)` reuse like CSV.

mod chrome;
mod content;
mod draw;

pub use chrome::visible_tabs;
pub use content::*;
pub use draw::*;
