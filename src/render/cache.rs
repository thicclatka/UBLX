//! Right-pane viewer: cache full styled [`Text`] + viewport windowing for heavy paths (delimiter
//! tables, large markdown). Lives under `render` with no `layout` dependency so [`UblxState`] can
//! hold [`ViewerTextCacheEntry`] via `crate::render::viewer_cache`.

use ratatui::text::Text;

/// Minimum raw UTF-8 size to cache parsed markdown + `to_text` output (viewport slice on scroll).
pub const VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES: usize = 64 * 1024;

/// One slot: path, layout width, theme, content identity, metrics, full rendered body.
#[derive(Clone)]
pub struct ViewerTextCacheEntry {
    pub path: String,
    pub content_width: u16,
    pub theme_name: String,
    pub content_ptr: usize,
    pub content_len: usize,
    pub line_count: usize,
    pub text: Text<'static>,
}

impl ViewerTextCacheEntry {
    /// Same file, width, theme, and viewer buffer as when the entry was built.
    #[must_use]
    pub fn matches(&self, path: &str, width: u16, theme: &str, raw: &str) -> bool {
        self.path == path
            && self.content_width == width
            && self.theme_name == theme
            && self.content_ptr == raw.as_ptr() as usize
            && self.content_len == raw.len()
    }

    /// Lines visible in the preview: `[scroll_y, scroll_y + viewport_h)` into cached `Text`.
    #[must_use]
    pub fn viewport_text(&self, scroll_y: u16, viewport_h: u16) -> Text<'static> {
        let lines = &self.text.lines;
        let n = lines.len();
        let start = (scroll_y as usize).min(n);
        let vh = viewport_h.max(1) as usize;
        let end = (start + vh).min(n);
        if start >= n {
            return Text::default();
        }
        Text::from(lines[start..end].to_vec())
    }
}
