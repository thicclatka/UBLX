//! Right pane: tabs, viewer / templates / metadata / writing content, fullscreen.
//!
//! ## Viewer pipeline (width, wrap, cache)
//!
//! 1. **Text width** — After vertical padding, [`scrollable_content::area_above_bottom_pad`], we
//!    iterate (up to 6 times): estimate total lines at width `text_w` →
//!    [`scrollable_content::viewport_text_width`] may shrink width to reserve space for a vertical
//!    scrollbar → repeat until stable. All layout and line counting for the viewer must use this
//!    final `text_w`.
//! 2. **Viewer text cache** — [`ensure_viewer_text_cache`] fills [`UblxState::viewer_text_cache`]
//!    for delimiter tables (always) and **large** markdown (see
//!    [`crate::render::cache::VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES`]). Scrolling uses a
//!    **viewport slice** into cached [`Text`] (see [`content_display_text`]).
//! 3. **Total lines** — [`viewer_total_lines`] must match what is actually drawn (scrollbar height).
//!    Branches: JSON metadata/writing → [`kv_tables::content_height`]; CSV → cache or
//!    [`csv::table_line_count`]; large markdown → cache line count; otherwise plain text → [`wrapped_line_count`].
//!    Delimited table preview: [`viewer_show_delimited_table`] — zahir category **CSV** (incl. tsv/tab/psv)
//!    **or** path extension `.csv`/`.tsv`/`.tab`/`.psv` so rows still get tables if metadata category is off.
//! 4. **Scroll** — [`scrollable_content::layout_scrollable_content`] + [`scrollable_content::draw_scrollbar`].
//! 5. **Draw** — If metadata/writing JSON is present, [`kv_tables::draw_tables`] (no `Paragraph`).
//!    Else [`content_display_text`] → `Paragraph` with optional ratatui [`Wrap`].
//! 6. **Images** — See [`crate::render::viewers::image::core`]: ≥512 KiB off-thread decode, tiered
//!    downscale, **min(tier, viewport cells)**, and a small **LRU** for instant back within a few files.
//!
//! **Preformatted layout:** [`viewer_uses_preformatted_layout`] is true for Markdown and for CSV
//! that parsed to a non-empty table. Those paths already produce viewport-width lines; ratatui must
//! *not* wrap again ([`ratatui_wrap_right_paragraph`]). For delimiter tables, when
//! [`ensure_viewer_text_cache`] has warmed the cache, the wrap check uses it instead of re-parsing.
//! Plain text and failed/empty CSV still use `Wrap`.
//!
//! ## Adding another category-based viewer
//!
//! Keep display, line count, and wrap flag aligned: extend [`viewer_display_text`] and
//! [`viewer_total_lines`], and set whether [`viewer_uses_preformatted_layout`] should skip `Wrap`.
//! Add a cache in state only if you need `(path, width)` reuse like CSV.

use ratatui::layout::{Constraint, Rect};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::handlers::zahir_ops::{ZahirFileType as FileType, delimiter_from_path_for_viewer};
use crate::layout::{
    setup::{RightPaneContent, RightPaneMode, UblxState, ViewData},
    style, themes,
};
use crate::render::{
    cache::{self, ViewerContentIdentity, ViewerTextCacheEntry},
    kv_tables, panes, scrollable_content,
    viewers::{csv_handler, image as viewer_image, markdown},
};
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::{format::StringObjTraits, format_bytes};

#[inline]
fn viewer_is_csv(rc: &RightPaneContent) -> bool {
    rc.viewer_zahir_type == Some(FileType::Csv)
}

/// Use comfy-table preview for delimiter-separated files: zahir **CSV** category **or** known extension.
/// Relying only on [`RightPaneContent::viewer_zahir_type`] breaks when the snapshot category string
/// doesn’t map to [`FileType::Csv`] (e.g. legacy `"Text"`), so we also key off the path.
#[inline]
fn viewer_show_delimited_table(rc: &RightPaneContent) -> bool {
    if viewer_is_markdown(rc) {
        return false;
    }
    if viewer_is_csv(rc) {
        return true;
    }
    rc.viewer_path
        .as_deref()
        .is_some_and(|p| delimiter_from_path_for_viewer(p).is_some())
}

#[inline]
fn viewer_is_markdown(rc: &RightPaneContent) -> bool {
    rc.viewer_zahir_type == Some(FileType::Markdown)
}

fn try_build_csv_cache_entry(
    path: &str,
    raw: &str,
    content_width: u16,
    theme_key: String,
    content_identity: ViewerContentIdentity,
) -> Option<ViewerTextCacheEntry> {
    let rows = csv_handler::parse_csv(raw, Some(path)).ok()?;
    if rows.is_empty() {
        return None;
    }
    let (table_string, line_count) = csv_handler::table_string_and_line_count(&rows, content_width);
    let text = csv_handler::table_string_to_text(&table_string);
    debug_assert_eq!(line_count, text.lines.len());
    Some(ViewerTextCacheEntry {
        path: path.to_string(),
        content_width,
        theme_name: theme_key,
        content_identity,
        line_count,
        text,
    })
}

fn try_build_markdown_cache_entry(
    path: &str,
    raw: &str,
    content_width: u16,
    theme_key: String,
) -> ViewerTextCacheEntry {
    let doc = markdown::parse_markdown(raw);
    let text = doc.to_text(content_width);
    let line_count = text.lines.len();
    ViewerTextCacheEntry {
        path: path.to_string(),
        content_width,
        theme_name: theme_key,
        content_identity: ViewerContentIdentity::BufferPtr {
            ptr: raw.as_ptr() as usize,
            len: raw.len(),
        },
        line_count,
        text,
    }
}

/// Warm delimiter-table LRU and [`UblxState::viewer_text_cache`] for large markdown (path + width + theme + buffer).
fn ensure_viewer_text_cache(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) {
    if state.right_pane_mode != RightPaneMode::Viewer {
        state.viewer_text_cache = None;
        return;
    }
    let Some(path) = right_content.viewer_path.as_deref() else {
        state.viewer_text_cache = None;
        return;
    };
    let Some(raw) = right_content.viewer.as_deref() else {
        state.viewer_text_cache = None;
        return;
    };
    let theme_key = themes::current().name.to_string();
    let theme = themes::current().name;

    if viewer_show_delimited_table(right_content) {
        state.viewer_text_cache = None;
        let key = cache::viewer_table_cache_key(
            path,
            content_width,
            theme,
            raw,
            right_content.viewer_mtime_ns,
            right_content.viewer_byte_size,
        );
        if state.csv_table_text_lru.get(&key).is_some() {
            return;
        }
        let Some(entry) =
            try_build_csv_cache_entry(path, raw, content_width, theme_key, key.identity.clone())
        else {
            return;
        };
        state.csv_table_text_lru.insert(key, entry);
        return;
    }

    if viewer_is_markdown(right_content) && raw.len() >= cache::VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES
    {
        if let Some(ref e) = state.viewer_text_cache
            && e.matches(path, content_width, theme, raw)
        {
            return;
        }
        state.viewer_text_cache = Some(try_build_markdown_cache_entry(
            path,
            raw,
            content_width,
            theme_key,
        ));
        return;
    }

    state.viewer_text_cache = None;
}

fn csv_cached_entry<'a>(
    state: &'a mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
    theme: &str,
) -> Option<&'a ViewerTextCacheEntry> {
    let path = right_content.viewer_path.as_deref()?;
    let raw = right_content.viewer.as_deref()?;
    let key = cache::viewer_table_cache_key(
        path,
        content_width,
        theme,
        raw,
        right_content.viewer_mtime_ns,
        right_content.viewer_byte_size,
    );
    state.csv_table_text_lru.get(&key)
}

fn viewer_text_cache_viewport_active(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) -> bool {
    let Some(vp) = right_content.viewer_path.as_deref() else {
        return false;
    };
    let Some(raw) = right_content.viewer.as_deref() else {
        return false;
    };
    let theme = themes::current().name;
    if viewer_show_delimited_table(right_content) {
        return csv_cached_entry(state, right_content, content_width, theme).is_some();
    }
    state.viewer_text_cache.as_ref().is_some_and(|e| {
        e.matches(vp, content_width, theme, raw)
            && viewer_is_markdown(right_content)
            && raw.len() >= cache::VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES
    })
}

/// Total line count for the right-pane content (used for scroll height).
fn viewer_total_lines(
    right_content: &RightPaneContent,
    content_width: u16,
    use_kv_tables: Option<&str>,
    state: &mut UblxState,
) -> usize {
    match (state.right_pane_mode, use_kv_tables) {
        (_, Some(json)) => kv_tables::content_height(json) as usize,
        (RightPaneMode::Viewer, _) => {
            let theme = themes::current().name;
            if viewer_show_delimited_table(right_content) {
                if let Some(e) = csv_cached_entry(state, right_content, content_width, theme) {
                    return e.line_count;
                }
            } else if let (Some(vp), Some(raw)) = (
                right_content.viewer_path.as_deref(),
                right_content.viewer.as_deref(),
            ) && let Some(ref e) = state.viewer_text_cache
                && e.matches(vp, content_width, theme, raw)
            {
                return e.line_count;
            }
            if viewer_show_delimited_table(right_content)
                && let Some(raw) = right_content.viewer.as_deref()
                && let Some(vp) = right_content.viewer_path.as_deref()
                && let Ok(rows) = csv_handler::parse_csv(raw, Some(vp))
                && !rows.is_empty()
            {
                return csv_handler::table_line_count(&rows, content_width);
            }
            if viewer_image::is_raster_preview_category(right_content) {
                if state.viewer_image.protocol.is_some() {
                    return 1;
                }
                if state.viewer_image.decode_rx.is_some() && state.viewer_image.err.is_none() {
                    return wrapped_line_count(
                        &viewer_image::raster_preview_label_body(
                            right_content,
                            UI_STRINGS.loading.general,
                        ),
                        content_width,
                    ) as usize;
                }
                if let Some(e) = state.viewer_image.err.as_deref() {
                    return wrapped_line_count(
                        &viewer_image::raster_preview_label_body(right_content, e),
                        content_width,
                    ) as usize;
                }
                let msg = right_content.viewer.as_deref().unwrap_or("");
                return wrapped_line_count(
                    &viewer_image::raster_preview_label_body(right_content, msg),
                    content_width,
                ) as usize;
            }
            if viewer_is_markdown(right_content) {
                let raw = right_content.viewer.as_deref().unwrap_or("");
                let doc = markdown::parse_markdown(raw);
                return doc.to_text(content_width).lines.len();
            }
            right_content
                .viewer
                .as_deref()
                .map_or(0, |t| wrapped_line_count(t, content_width) as usize)
        }
        (RightPaneMode::Templates, _) => right_content.templates.lines().count(),
        (RightPaneMode::Writing | RightPaneMode::Metadata, _) => 0,
    }
}

/// Markdown and successful delimiter tables are fully laid out to the viewport; ratatui must not re-wrap.
fn viewer_uses_preformatted_layout(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) -> bool {
    if state.right_pane_mode != RightPaneMode::Viewer {
        return false;
    }
    if right_content.viewer_path.is_none() {
        return false;
    }
    if viewer_is_markdown(right_content) {
        return true;
    }
    if viewer_image::is_raster_preview_category(right_content)
        && state.viewer_image.protocol.is_some()
    {
        return true;
    }
    if !viewer_show_delimited_table(right_content) {
        return false;
    }
    let theme = themes::current().name;
    if csv_cached_entry(state, right_content, content_width, theme).is_some() {
        return true;
    }
    let Some(raw) = right_content.viewer.as_deref() else {
        return false;
    };
    csv_handler::parse_csv(raw, right_content.viewer_path.as_deref())
        .map(|r| !r.is_empty())
        .unwrap_or(false)
}

fn ratatui_wrap_right_paragraph(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
) -> bool {
    !viewer_uses_preformatted_layout(state, right_content, content_width)
}

fn wrapped_line_count(text: &str, width: u16) -> u16 {
    let w = width as usize;
    if w == 0 {
        return 0;
    }
    text.lines()
        .map(|line| (line.chars().count().div_ceil(w)).max(1))
        .sum::<usize>()
        .min(u16::MAX as usize) as u16
}

fn viewer_display_text(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
    text_viewport: Option<(u16, u16)>,
) -> Text<'static> {
    let raw = right_content
        .viewer
        .as_deref()
        .unwrap_or(UI_STRINGS.pane.viewer_placeholder);
    if right_content.viewer_path.is_some() {
        if viewer_show_delimited_table(right_content) {
            let theme = themes::current().name;
            if let Some(e) = csv_cached_entry(state, right_content, content_width, theme) {
                return match text_viewport {
                    Some((sy, vh)) => e.viewport_text(sy, vh),
                    None => e.text.clone(),
                };
            }
            if let Ok(rows) = csv_handler::parse_csv(raw, right_content.viewer_path.as_deref())
                && !rows.is_empty()
            {
                return csv_handler::table_to_text(&rows, content_width);
            }
            // Parse failed or empty: fall back to raw
        } else if viewer_is_markdown(right_content) {
            let theme = themes::current().name;
            if raw.len() >= cache::VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES
                && let Some(ref path) = right_content.viewer_path
                && let Some(ref e) = state.viewer_text_cache
                && e.matches(path.as_str(), content_width, theme, raw)
            {
                return match text_viewport {
                    Some((sy, vh)) => e.viewport_text(sy, vh),
                    None => e.text.clone(),
                };
            }
            let doc = markdown::parse_markdown(raw);
            return doc.to_text(content_width);
        } else if viewer_image::is_raster_preview_category(right_content) {
            if state.viewer_image.protocol.is_some() {
                return Text::default();
            }
            if state.viewer_image.decode_rx.is_some() && state.viewer_image.err.is_none() {
                return Text::from(viewer_image::raster_preview_label_body(
                    right_content,
                    UI_STRINGS.loading.general,
                ));
            }
            if let Some(e) = state.viewer_image.err.as_deref() {
                return Text::from(viewer_image::raster_preview_label_body(right_content, e));
            }
            let msg = right_content.viewer.as_deref().unwrap_or("");
            return Text::from(viewer_image::raster_preview_label_body(right_content, msg));
        }
    }
    Text::from(raw.to_string())
}

fn content_display_text(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    content_width: u16,
    text_viewport: Option<(u16, u16)>,
) -> Text<'static> {
    match state.right_pane_mode {
        RightPaneMode::Viewer => {
            viewer_display_text(state, right_content, content_width, text_viewport)
        }
        RightPaneMode::Templates => ratatui::text::Text::from(right_content.templates.clone()),
        RightPaneMode::Metadata => ratatui::text::Text::from(
            right_content
                .metadata
                .clone()
                .unwrap_or_else(|| UI_STRINGS.pane.not_available.to_string()),
        ),
        RightPaneMode::Writing => ratatui::text::Text::from(
            right_content
                .writing
                .clone()
                .unwrap_or_else(|| UI_STRINGS.pane.not_available.to_string()),
        ),
    }
}

fn title(state: &UblxState) -> String {
    let label = match state.right_pane_mode {
        RightPaneMode::Viewer => UI_STRINGS.pane.viewer,
        RightPaneMode::Templates => UI_STRINGS.pane.templates,
        RightPaneMode::Metadata => UI_STRINGS.pane.metadata,
        RightPaneMode::Writing => UI_STRINGS.pane.writing,
    };
    UI_STRINGS.pad(label)
}

fn visible_tabs(right_content: &RightPaneContent) -> Vec<(RightPaneMode, &'static str)> {
    [
        (RightPaneMode::Viewer, UI_STRINGS.pane.tab_viewer),
        (RightPaneMode::Templates, UI_STRINGS.pane.tab_templates),
        (RightPaneMode::Metadata, UI_STRINGS.pane.tab_metadata),
        (RightPaneMode::Writing, UI_STRINGS.pane.tab_writing),
    ]
    .into_iter()
    .filter(|(mode, _)| match mode {
        RightPaneMode::Viewer => true,
        RightPaneMode::Templates => !right_content.templates.is_empty(),
        RightPaneMode::Metadata => right_content.metadata.is_some(),
        RightPaneMode::Writing => right_content.writing.is_some(),
    })
    .collect()
}

/// Sync PDF page picker, then optional bottom title line (size, mtime, PDF page).
fn right_pane_footer_line(
    state: &mut UblxState,
    right_content: &RightPaneContent,
) -> Option<Line<'static>> {
    viewer_image::sync_pdf_selection_state(state, right_content);
    let pdf_footer = viewer_image::pdf_page_footer_text(right_content, &state.viewer_image);
    let show_footer = state.right_pane_mode == RightPaneMode::Viewer
        && (right_content.viewer_byte_size.is_some()
            || right_content.viewer_mtime_ns.is_some()
            || pdf_footer.is_some());
    let size_str = right_content.viewer_byte_size.map(format_bytes);
    show_footer
        .then(|| {
            style::viewer_footer_line(
                size_str.as_deref(),
                right_content.viewer_mtime_ns,
                pdf_footer.as_deref(),
            )
        })
        .flatten()
}

/// Fullscreen footer includes middle-pane counter/sort nodes plus viewer footer nodes.
fn right_pane_footer_line_fullscreen(
    state: &mut UblxState,
    right_content: &RightPaneContent,
    view: &ViewData,
) -> Option<Line<'static>> {
    let mut spans = panes::middle::line_for(
        state.panels.content_state.selected(),
        view.content_len,
        state.main_mode,
        state.panels.content_sort,
    )
    .spans;
    if let Some(viewer_line) = right_pane_footer_line(state, right_content) {
        spans.extend(viewer_line.spans);
    }
    Some(Line::from(spans).right_aligned())
}

fn right_pane_block<'a>(
    top_title: Option<&'a str>,
    footer_line: Option<&'a Line<'static>>,
) -> Block<'a> {
    let b = Block::default()
        .borders(Borders::ALL)
        .style(style::text_style());
    let b = match top_title {
        Some(t) => b.title(t),
        None => b,
    };
    match footer_line {
        Some(line) => b.title_bottom(line.clone()),
        None => b,
    }
}

/// Scroll layout, KV tables / raster / paragraph, and scrollbar. Shared by [`draw_right_pane`] and [`draw_right_pane_fullscreen`].
fn draw_right_pane_scrollable_body(
    f: &mut ratatui::Frame,
    state: &mut UblxState,
    right_content: &RightPaneContent,
    scroll_area: Rect,
    bottom_pad: u16,
) {
    let padded = scrollable_content::area_above_bottom_pad(scroll_area, bottom_pad);
    let use_kv_tables = match state.right_pane_mode {
        RightPaneMode::Metadata => right_content.metadata.as_deref(),
        RightPaneMode::Writing => right_content.writing.as_deref(),
        _ => None,
    };
    viewer_image::ensure_viewer_image(state, right_content, Some((padded.width, padded.height)));
    let mut text_w = padded.width;
    for _ in 0..6 {
        ensure_viewer_text_cache(state, right_content, text_w);
        let guess_lines = viewer_total_lines(right_content, text_w, use_kv_tables, state);
        let w_next = scrollable_content::viewport_text_width(padded, guess_lines);
        if w_next == text_w {
            break;
        }
        text_w = w_next;
    }
    ensure_viewer_text_cache(state, right_content, text_w);
    let total_lines = viewer_total_lines(right_content, text_w, use_kv_tables, state);
    let layout = scrollable_content::layout_scrollable_content(
        scroll_area,
        total_lines,
        &mut state.panels.preview_scroll,
        bottom_pad,
    );
    let text_rect = layout.content_rect;
    if let Some(json) = use_kv_tables {
        kv_tables::draw_tables(f, text_rect, json, layout.scroll_y);
    } else {
        let image_mode = state.right_pane_mode == RightPaneMode::Viewer
            && viewer_image::is_raster_preview_category(right_content)
            && right_content.viewer_path.is_some()
            && state.viewer_image.protocol.is_some();
        if image_mode {
            if let Some(proto) = state.viewer_image.protocol.as_mut() {
                f.render_stateful_widget(viewer_image::stateful_widget(), text_rect, proto);
                let _ = proto.last_encoding_result();
            }
        } else {
            let text_vp = viewer_text_cache_viewport_active(state, right_content, text_w)
                .then_some((layout.scroll_y, text_rect.height));
            let para_scroll = if text_vp.is_some() {
                (0, 0)
            } else {
                (layout.scroll_y, 0)
            };
            let mut paragraph =
                Paragraph::new(content_display_text(state, right_content, text_w, text_vp))
                    .style(style::text_style())
                    .scroll(para_scroll);
            if ratatui_wrap_right_paragraph(state, right_content, text_w) {
                paragraph = paragraph.wrap(Wrap { trim: false });
            }
            f.render_widget(paragraph, text_rect);
        }
    }
    scrollable_content::draw_scrollbar(f, &layout, total_lines);
}

/// Draw the right (viewer) pane. `chunks` must have at least 3 elements; the right pane uses `chunks[2]`.
pub fn draw_right_pane(
    f: &mut ratatui::Frame,
    state: &mut UblxState,
    right_content: &RightPaneContent,
    chunks: &[Rect],
) {
    let area = chunks[2];
    let footer_line = right_pane_footer_line(state, right_content);
    let right_block = right_pane_block(None, footer_line.as_ref());
    let tabs = visible_tabs(right_content);
    if !tabs.is_empty() && !tabs.iter().any(|(m, _)| *m == state.right_pane_mode) {
        state.right_pane_mode = tabs[0].0;
    }
    let tab_spans: Vec<Span> = tabs
        .iter()
        .flat_map(|(mode, label)| style::tab_node_segment(label, *mode == state.right_pane_mode))
        .collect();
    let right_inner = right_block.inner(area);
    let constraints = &[
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ][..];
    let right_split = style::split_vertical(right_inner, constraints);
    let tab_row_chunks = style::tab_row_padded(right_split[0]);
    let content_chunks = style::tab_row_padded(right_split[2]);

    f.render_widget(&right_block, area);
    f.render_widget(Paragraph::new(Line::from(tab_spans)), tab_row_chunks[1]);

    let content_area = content_chunks[1];
    let bottom_pad = UI_CONSTANTS.v_pad;
    draw_right_pane_scrollable_body(f, state, right_content, content_area, bottom_pad);
}

/// Draw the current right-pane tab in full screen (hide categories and contents). Esc to exit.
pub fn draw_right_pane_fullscreen(
    f: &mut ratatui::Frame,
    state: &mut UblxState,
    right_content: &RightPaneContent,
    view: &crate::layout::setup::ViewData,
    area: Rect,
) {
    let footer_line = right_pane_footer_line_fullscreen(state, right_content, view);
    let fullscreen_title = format!("{} {}", title(state), UI_STRINGS.brand.fullscreen_suffix);
    let block = right_pane_block(Some(fullscreen_title.as_str()), footer_line.as_ref());
    let inner = block.inner(area);
    let bottom_pad = UI_CONSTANTS.v_pad;
    f.render_widget(&block, area);
    draw_right_pane_scrollable_body(f, state, right_content, inner, bottom_pad);
}
