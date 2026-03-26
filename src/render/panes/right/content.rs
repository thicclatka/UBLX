//! Right-pane scroll **content**: [`content_display_text`] for every tab, plus **Viewer**-tab helpers
//! (CSV / markdown / raster labels, [`ensure_viewer_text_cache`], line counts, wrap).

use ratatui::text::Text;

use crate::engine::cache::{self, ViewerContentIdentity, ViewerTextCacheEntry};
use crate::integrations::{ZahirFileType as FileType, delimiter_from_path_for_viewer};
use crate::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use crate::render::viewers::{csv_handler, image as viewer_image, markdown};
use crate::themes;
use crate::ui::UI_STRINGS;

#[inline]
fn viewer_is_csv(rc: &RightPaneContent) -> bool {
    rc.viewer_zahir_type == Some(FileType::Csv)
}

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

pub fn ensure_viewer_text_cache(
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

pub fn viewer_text_cache_viewport_active(
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
    state
        .viewer_text_cache
        .as_ref()
        .is_some_and(|e: &ViewerTextCacheEntry| {
            e.matches(vp, content_width, theme, raw)
                && viewer_is_markdown(right_content)
                && raw.len() >= cache::VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES
        })
}

pub fn viewer_total_lines(
    right_content: &RightPaneContent,
    content_width: u16,
    use_kv_tables: Option<&str>,
    state: &mut UblxState,
) -> usize {
    match (state.right_pane_mode, use_kv_tables) {
        (_, Some(json)) => crate::render::kv_tables::content_height(json) as usize,
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

pub fn ratatui_wrap_right_paragraph(
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

pub fn content_display_text(
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
