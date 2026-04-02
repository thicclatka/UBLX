//! Right-pane viewer: cache full styled [`Text`] + viewport windowing for heavy paths (delimiter
//! tables, large markdown, large syntect-highlighted files). Lives under `render` with no `layout`
//! dependency so [`UblxState`] can hold cache fields via `crate::render::cache`.

use ratatui::text::Text;

use crate::engine::db_ops::UblxDbCategory;
use crate::themes::Appearance;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewerTextCache {
    pub min_markdown_bytes: usize,
    /// Syntect-backed viewers (code, JSON, log, …): smaller bodies highlight on the UI thread; at or above this size we use async + cache.
    pub min_syntect_bytes: usize,
    /// Delimiter-table viewer: offload parse + layout when raw is at least this large.
    pub min_csv_bytes: usize,
    pub csv_lru_cap: usize,
}

const KB_TO_BYTES_CONVERSION: usize = 1024;

impl ViewerTextCache {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min_markdown_bytes: 64 * KB_TO_BYTES_CONVERSION,
            min_syntect_bytes: 64 * KB_TO_BYTES_CONVERSION,
            min_csv_bytes: 32 * KB_TO_BYTES_CONVERSION,
            csv_lru_cap: 3,
        }
    }
}

impl Default for ViewerTextCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Thresholds for markdown / syntect viewer caching. Use this in `const` contexts; [`Default`] matches.
pub const VIEWER_TEXT_CACHE: ViewerTextCache = ViewerTextCache::new();

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
        Self::with_capacity(ViewerTextCache::new().csv_lru_cap)
    }
}

// -----------------------------------------------------------------------------
// Viewer text cache entries
// -----------------------------------------------------------------------------

/// Whether a cached render still matches the live preview.
///
/// We **do not** store raw pointers: the preview `String` is rebuilt frequently (`clone` / disk
/// re-read), so pointer equality is never stable across frames. Use DB mtime + `raw.len()` when
/// mtime exists; otherwise length only (weaker).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewerContentIdentity {
    /// `mtime_ns` from the snapshot row; `len` is [`str::len`] when the entry was built.
    MtimeAndLen { mtime_ns: i64, len: usize },
    /// Snapshot had no mtime: match on length only.
    LenOnly { len: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerTableCacheKey {
    pub path: String,
    pub content_width: u16,
    pub theme_name: String,
    pub identity: ViewerContentIdentity,
}

/// Derive identity for cache invalidation. Call with the current preview `raw` and DB mtime.
#[must_use]
pub fn viewer_content_identity(raw: &str, viewer_mtime_ns: Option<i64>) -> ViewerContentIdentity {
    match viewer_mtime_ns {
        Some(mtime_ns) => ViewerContentIdentity::MtimeAndLen {
            mtime_ns,
            len: raw.len(),
        },
        None => ViewerContentIdentity::LenOnly { len: raw.len() },
    }
}

#[must_use]
pub fn viewer_table_cache_key(
    path: &str,
    content_width: u16,
    theme_name: &str,
    raw: &str,
    viewer_mtime_ns: Option<i64>,
) -> ViewerTableCacheKey {
    ViewerTableCacheKey {
        path: path.to_string(),
        content_width,
        theme_name: theme_name.to_string(),
        identity: viewer_content_identity(raw, viewer_mtime_ns),
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

/// Snapshot + palette context for syntect viewer cache match and build (`content` module builds entries).
#[derive(Clone, Copy, Debug)]
pub struct CodeViewerCacheParams<'a> {
    pub path: &'a str,
    pub raw: &'a str,
    pub content_width: u16,
    /// Palette name (`themes::current().name`); build clones to owned [`ViewerTextCacheEntry::theme_name`].
    pub theme_name: &'a str,
    pub appearance: Appearance,
    pub category: UblxDbCategory,
    pub mtime_ns: Option<i64>,
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
    /// `Some` when this entry was built with syntect; [`None`] for markdown and delimiter tables.
    pub syntect: Option<(Appearance, UblxDbCategory)>,
}

impl ViewerTextCacheEntry {
    /// Same file, width, theme, and viewer buffer as when the entry was built (buffer identity).
    #[must_use]
    pub fn matches(&self, path: &str, width: u16, theme: &str, raw: &str) -> bool {
        self.matches_with_file_meta(path, width, theme, raw, None)
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
    ) -> bool {
        if self.path != path || self.content_width != width || self.theme_name != theme {
            return false;
        }
        match &self.content_identity {
            ViewerContentIdentity::MtimeAndLen { mtime_ns, len } => {
                viewer_mtime_ns == Some(*mtime_ns) && raw.len() == *len
            }
            ViewerContentIdentity::LenOnly { len } => raw.len() == *len,
        }
    }

    /// Markdown cache entry (not syntect).
    #[must_use]
    pub fn matches_markdown_viewer(&self, path: &str, width: u16, theme: &str, raw: &str) -> bool {
        self.syntect.is_none() && self.matches(path, width, theme, raw)
    }

    /// Code-highlighted cache: same appearance + DB category + buffer/file identity.
    #[must_use]
    pub fn matches_syntect_viewer(&self, p: &CodeViewerCacheParams<'_>) -> bool {
        self.syntect == Some((p.appearance, p.category))
            && self.matches_with_file_meta(p.path, p.content_width, p.theme_name, p.raw, p.mtime_ns)
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
