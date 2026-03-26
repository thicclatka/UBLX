//! Right-pane viewer: cache full styled [`Text`] + viewport windowing for heavy paths (delimiter
//! tables, large markdown). Lives under `render` with no `layout` dependency so [`UblxState`] can
//! hold cache fields via `crate::render::cache`.

use ratatui::text::Text;

/// Minimum raw UTF-8 size to cache parsed markdown + `to_text` output (viewport slice on scroll).
pub const VIEWER_TEXT_CACHE_MIN_MARKDOWN_BYTES: usize = 64 * 1024;

/// Max delimiter-table previews kept in [`crate::layout::setup::UblxState::csv_table_text_lru`].
pub const CSV_VIEWER_TEXT_LRU_CAP: usize = 3;

// -----------------------------------------------------------------------------
// Generic LRU (small N: linear scan is fine)
// -----------------------------------------------------------------------------

/// Least-recently-used by **recency of use** (get/insert move to MRU). Index `0` is most recent.
#[derive(Debug)]
pub struct LruCache<K, V> {
    pub cap: usize,
    pub entries: Vec<(K, V)>,
}

impl<K: PartialEq, V> LruCache<K, V> {
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            entries: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// MRU at index 0. Promotes a hit to the front.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let i = self.entries.iter().position(|(k, _)| k == key)?;
        let (k, v) = self.entries.remove(i);
        self.entries.insert(0, (k, v));
        Some(&mut self.entries[0].1)
    }

    /// Promotes a hit to the front.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.get_mut(key).map(|v| &*v)
    }

    /// Insert or replace at MRU; drops LRU entries past `cap`.
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(i) = self.entries.iter().position(|(k, _)| k == &key) {
            self.entries.remove(i);
        }
        self.entries.insert(0, (key, value));
        while self.entries.len() > self.cap {
            self.entries.pop();
        }
    }
}

impl<K: PartialEq, V> Default for LruCache<K, V> {
    fn default() -> Self {
        Self::with_capacity(CSV_VIEWER_TEXT_LRU_CAP)
    }
}

// -----------------------------------------------------------------------------
// Viewer text cache entries
// -----------------------------------------------------------------------------

/// Stable identity for invalidation: buffer pointer (same session) or snapshot file revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewerContentIdentity {
    BufferPtr { ptr: usize, len: usize },
    FileRevision { mtime_ns: i64, byte_size: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerTableCacheKey {
    pub path: String,
    pub content_width: u16,
    pub theme_name: String,
    pub identity: ViewerContentIdentity,
}

/// Derive the same identity used when building a [`ViewerTextCacheEntry`] for delimiter tables.
#[must_use]
pub fn viewer_content_identity(
    raw: &str,
    viewer_mtime_ns: Option<i64>,
    viewer_byte_size: Option<u64>,
) -> ViewerContentIdentity {
    match (viewer_mtime_ns, viewer_byte_size) {
        (Some(m), Some(s)) if raw.len() as u64 == s => ViewerContentIdentity::FileRevision {
            mtime_ns: m,
            byte_size: s,
        },
        _ => ViewerContentIdentity::BufferPtr {
            ptr: raw.as_ptr() as usize,
            len: raw.len(),
        },
    }
}

#[must_use]
pub fn viewer_table_cache_key(
    path: &str,
    content_width: u16,
    theme_name: &str,
    raw: &str,
    viewer_mtime_ns: Option<i64>,
    viewer_byte_size: Option<u64>,
) -> ViewerTableCacheKey {
    ViewerTableCacheKey {
        path: path.to_string(),
        content_width,
        theme_name: theme_name.to_string(),
        identity: viewer_content_identity(raw, viewer_mtime_ns, viewer_byte_size),
    }
}

#[must_use]
pub fn viewer_table_cache_key_from_entry(e: &ViewerTextCacheEntry) -> ViewerTableCacheKey {
    ViewerTableCacheKey {
        path: e.path.clone(),
        content_width: e.content_width,
        theme_name: e.theme_name.clone(),
        identity: e.content_identity.clone(),
    }
}

/// One slot: path, layout width, theme, content identity, metrics, full rendered body.
#[derive(Clone)]
pub struct ViewerTextCacheEntry {
    pub path: String,
    pub content_width: u16,
    pub theme_name: String,
    pub content_identity: ViewerContentIdentity,
    pub line_count: usize,
    pub text: Text<'static>,
}

impl ViewerTextCacheEntry {
    /// Same file, width, theme, and viewer buffer as when the entry was built (buffer identity).
    #[must_use]
    pub fn matches(&self, path: &str, width: u16, theme: &str, raw: &str) -> bool {
        self.matches_with_file_meta(path, width, theme, raw, None, None)
    }

    /// Prefer for delimiter tables when snapshot supplies mtime/size (stable across buffer realloc).
    #[must_use]
    pub fn matches_with_file_meta(
        &self,
        path: &str,
        width: u16,
        theme: &str,
        raw: &str,
        viewer_mtime_ns: Option<i64>,
        viewer_byte_size: Option<u64>,
    ) -> bool {
        if self.path != path || self.content_width != width || self.theme_name != theme {
            return false;
        }
        match &self.content_identity {
            ViewerContentIdentity::FileRevision {
                mtime_ns,
                byte_size,
            } => {
                viewer_mtime_ns == Some(*mtime_ns)
                    && viewer_byte_size == Some(*byte_size)
                    && raw.len() as u64 == *byte_size
            }
            ViewerContentIdentity::BufferPtr { ptr, len } => {
                *ptr == raw.as_ptr() as usize && *len == raw.len()
            }
        }
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
