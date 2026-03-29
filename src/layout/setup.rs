//! 3-panel TUI: categories (left), contents (middle), preview (right).
//!
//! [`crate::handlers::core::run_tui_session`] drives the loop; work per tick is split into four phases (see classification below).
//! Action application (key → state changes) lives in [`crate::handlers::state_transitions`].

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use ratatui::style::Style;
use ratatui::widgets::ListState;

use super::style;

use crate::engine::{cache, db_ops::DeltaType};
use crate::integrations::ZahirFileType as FileType;
use crate::render::viewers::pdf_preview::PDFPrefetch;
use crate::utils::{ClipboardCopyCommand, ToastSlot};

/// Re-export snapshot row type for layout/view/render (`path`, category, size).
pub use crate::engine::db_ops::SnapshotTuiRow as TuiRow;

/// Category string for directories in the snapshot (matches [`crate::engine::db_ops::UblxDbCategory`]).
pub const CATEGORY_DIRECTORY: &str = "Directory";

/// List panels: categories, contents, focus, preview scroll, and highlight style.
#[derive(Default)]
pub struct PanelState {
    pub category_state: ListState,
    pub content_state: ListState,
    pub focus: PanelFocus,
    pub preview_scroll: u16,
    pub prev_preview_key: Option<(usize, Option<usize>)>,
    pub highlight_style: Style,
    pub content_sort: ContentSort,
    /// Temporary anchor used to keep the same selected item identity after sort changes.
    pub sort_anchor_path: Option<String>,
}

impl PanelState {
    fn new() -> Self {
        let mut p = Self {
            highlight_style: style::list_highlight(),
            ..Default::default()
        };
        p.category_state.select(Some(0));
        p.content_state.select(Some(0));
        p
    }
}

/// Middle-pane sort direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

impl SortDirection {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

/// Snapshot/Duplicates middle-pane sort key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SnapshotSortKey {
    #[default]
    Name,
    Size,
    Mod,
}

impl SnapshotSortKey {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Size,
            Self::Size => Self::Mod,
            Self::Mod => Self::Name,
        }
    }
}

/// Mode-aware content sort state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContentSort {
    pub snapshot_key: SnapshotSortKey,
    pub snapshot_dir: SortDirection,
    pub delta_dir: SortDirection,
}

impl ContentSort {
    #[must_use]
    pub fn cycle_for_mode(self, main_mode: MainMode) -> Self {
        match main_mode {
            MainMode::Snapshot | MainMode::Duplicates => {
                if self.snapshot_dir == SortDirection::Asc {
                    Self {
                        snapshot_dir: SortDirection::Desc,
                        ..self
                    }
                } else {
                    Self {
                        snapshot_key: self.snapshot_key.next(),
                        snapshot_dir: SortDirection::Asc,
                        ..self
                    }
                }
            }
            MainMode::Delta => Self {
                delta_dir: self.delta_dir.next(),
                ..self
            },
            MainMode::Lenses | MainMode::Settings => self,
        }
    }
}

/// Search bar state.
#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub active: bool,
}

/// Theme selector and override.
#[derive(Default)]
pub struct ThemeState {
    pub selector_visible: bool,
    pub selector_index: usize,
    pub before_selector: Option<String>,
    pub override_name: Option<String>,
}

/// Toast notifications stack and per-operation consumed counts.
#[derive(Default)]
pub struct ToastState {
    pub slots: Vec<ToastSlot>,
    pub consumed_per_operation: HashMap<String, usize>,
}

/// Open (Terminal/GUI) menu state.
#[derive(Default)]
pub struct OpenMenuState {
    pub visible: bool,
    pub path: Option<String>,
    pub can_terminal: bool,
    pub selected_index: usize,
}

/// Lens menu (Add to lens) state.
#[derive(Default)]
pub struct LensMenuState {
    pub visible: bool,
    pub path: Option<String>,
    pub selected_index: usize,
    pub name_input: Option<String>,
}

/// Spacebar context menu state.
#[derive(Default)]
pub struct SpaceMenuState {
    pub visible: bool,
    pub selected_index: usize,
    pub kind: Option<SpaceMenuKind>,
}

/// After Space → Enhance policy: choose auto / manual batch Zahir for this directory subtree (local TOML).
#[derive(Default)]
pub struct EnhancePolicyMenuState {
    pub visible: bool,
    pub path: Option<String>,
    pub selected_index: usize,
}

/// Lens rename input and delete-lens confirmation.
#[derive(Default)]
pub struct LensConfirmState {
    pub rename_input: Option<(String, String)>,
    pub delete_visible: bool,
    pub delete_lens_name: Option<String>,
    pub delete_selected: usize,
}

/// Help overlay and fullscreen right-pane preview.
#[derive(Default)]
pub struct ViewerChrome {
    pub help_visible: bool,
    pub viewer_fullscreen: bool,
}

/// First-run flow when the per-root DB was new: pick root, optional prior roots, then prior-settings or enhance-all.
#[derive(Debug, Clone)]
pub struct StartupPromptState {
    pub phase: StartupPromptPhase,
}

#[derive(Debug, Clone)]
pub enum StartupPromptPhase {
    /// Welcome + root picker: current dir first, then optional recent roots. See [`crate::render::overlays::popup::render_startup_welcome_root_choice`].
    RootChoice {
        selected_index: usize,
        roots: Vec<PathBuf>,
    },
    /// Prior settings for this folder: local `ublx.toml` / cache vs start clean. See [`crate::render::overlays::popup::render_startup_previous_settings_prompt`].
    /// 0 = use saved (copy cache → local when there is no local file), 1 = start fresh.
    PreviousSettings { selected_index: usize },
    /// Enable full-directory `ZahirScan` (`enable_enhance_all`). See [`crate::render::overlays::popup::render_startup_enhance_all_prompt`]. 0 = Yes, 1 = No.
    Enhance { selected_index: usize },
}

/// Background snapshot: user request, poll `.ublx_tmp` while running, and completion.
#[derive(Default)]
pub struct BackgroundSnapshot {
    pub requested: bool,
    pub poll_deadline: Option<std::time::Instant>,
    pub done_received: bool,
    /// After the in-flight snapshot finishes, run one more (e.g. `[[enhance_policy]]` = auto just saved).
    pub defer_snapshot_after_current: bool,
}

/// Lazy-load duplicate groups when the user opens the Duplicates tab.
#[derive(Default)]
pub struct DuplicateLoadGate {
    pub requested: bool,
}

/// First real frame vs later ticks; redraw after returning from external editor.
#[derive(Clone, Copy, Debug)]
pub struct SessionTickFlags {
    pub first_tick: bool,
    pub refresh_terminal_after_editor: bool,
}

impl Default for SessionTickFlags {
    fn default() -> Self {
        Self {
            first_tick: true,
            refresh_terminal_after_editor: false,
        }
    }
}

/// Snapshot table reload and one-shot dedup for the background full-enhance toast.
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionReloadFlags {
    /// After single-file `ZahirScan` enhance, reload snapshot rows from DB on next tick.
    pub snapshot_rows: bool,
    /// After we show the "enhancing in background" toast for [`crate::engine::orchestrator::should_force_full_zahir`], suppress duplicates until restart.
    pub force_full_enhance_toast_shown: bool,
}

/// One-shot session coordination for ticks, editor handoff, and DB reload.
#[derive(Default)]
pub struct SessionFlow {
    pub tick: SessionTickFlags,
    pub reload: SessionReloadFlags,
}

pub struct PDF {
    pub page: u32,
    pub page_count: Option<u32>,
    pub for_path: Option<PathBuf>,
    pub page_count_rx: Option<mpsc::Receiver<Result<u32, String>>>,
    pub prefetch_cancel: Arc<AtomicU64>,
    pub prefetch_earliest: Option<Instant>,
    pub prefetch_rx: Option<mpsc::Receiver<(String, Result<image::DynamicImage, String>)>>,
}

impl Default for PDF {
    fn default() -> Self {
        Self {
            page: 1,
            page_count: None,
            for_path: None,
            page_count_rx: None,
            prefetch_cancel: Arc::new(AtomicU64::new(0)),
            prefetch_earliest: None,
            prefetch_rx: None,
        }
    }
}

/// State for the image viewer in the right pane (`ratatui-image`, tiered downscale, optional background decode).
#[derive(Default)]
pub struct ViewerImageState {
    pub protocol: Option<ratatui_image::protocol::StatefulProtocol>,
    pub picker: Option<ratatui_image::picker::Picker>,
    /// Cache key: path display, or `path#pN` for PDF page `N`.
    pub key: Option<String>,
    /// When set, a background thread is decoding/downsizing; poll in [`crate::render::viewers::image::ensure_viewer_image`].
    pub decode_rx: Option<mpsc::Receiver<Result<image::DynamicImage, String>>>,
    pub err: Option<String>,
    /// Recent previews (not the current row). Size [`Self::LRU_CAP`] is tied to PDF prefetch (see [`ViewerImageState::LRU_CAP`]).
    pub image_lru: VecDeque<(String, ratatui_image::protocol::StatefulProtocol)>,
    /// PDF: one-based page; PDF: selected file this state applies to.
    pub pdf: PDF,
}

impl ViewerImageState {
    /// `PDFPrefetch::MAX_EXTRA_PAGES` prefetched PDFs (pages 2..) plus **four** slots to stash the previous page
    pub const LRU_EXTRA_SLOTS: usize = 4;
    pub const LRU_CAP: usize = PDFPrefetch::MAX_EXTRA_PAGES as usize + Self::LRU_EXTRA_SLOTS;

    /// Push a finished preview into the LRU ring; drops the oldest entry when full.
    pub fn push_lru(&mut self, path: String, proto: ratatui_image::protocol::StatefulProtocol) {
        while self.image_lru.len() >= Self::LRU_CAP {
            self.image_lru.pop_front();
        }
        self.image_lru.push_back((path, proto));
    }

    /// Remove and return a cached protocol for `path` if present.
    pub fn take_from_lru(
        &mut self,
        path: &str,
    ) -> Option<ratatui_image::protocol::StatefulProtocol> {
        let pos = self.image_lru.iter().position(|(k, _)| k == path)?;
        self.image_lru.remove(pos).map(|(_, proto)| proto)
    }

    /// Drop an LRU entry matching `key` so a prefetch can replace it.
    pub fn remove_lru_key(&mut self, key: &str) {
        if let Some(pos) = self.image_lru.iter().position(|(k, _)| k == key) {
            self.image_lru.remove(pos);
        }
    }

    /// Clear loaded image, error, and async decode channel; **retains** [`Self::picker`] so the
    /// terminal is not re-queried on every selection (matches previous flat-field behavior).
    /// Finished previews are moved into [`Self::image_lru`] so returning to an image can be instant.
    pub fn clear(&mut self) {
        self.pdf.prefetch_cancel.fetch_add(1, Ordering::SeqCst);
        self.pdf.prefetch_rx = None;
        self.pdf.prefetch_earliest = None;
        self.decode_rx = None;
        self.pdf.page_count_rx = None;
        self.err = None;
        let k = self.key.take();
        let p = self.protocol.take();
        if let (Some(k), Some(p)) = (k, p) {
            self.push_lru(k, p);
        }
        self.pdf.for_path = None;
        self.pdf.page = 1;
        self.pdf.page_count = None;
    }
}

/// Avoids re-reading the selected file every UI tick when path, category, size, and mtime match.
#[derive(Debug, Clone)]
pub struct ViewerDiskContentCache {
    pub rel_path: String,
    /// Snapshot category (drives file-type handling in the viewer).
    pub category: String,
    pub file_len: u64,
    pub modified: Option<std::time::SystemTime>,
    pub viewer_str: Option<String>,
    pub embedded_cover_raster: Option<Vec<u8>>,
    pub viewer_can_open: bool,
}

impl ViewerDiskContentCache {
    #[must_use]
    pub fn matches(&self, path: &str, category: &str, meta: &std::fs::Metadata) -> bool {
        self.rel_path == path
            && self.category == category
            && self.file_len == meta.len()
            && self.modified == meta.modified().ok()
    }
}

#[derive(Default)]
pub struct RightPaneAsync {
    pub generation: u64,
    pub last_spawn_path: String,
    pub displayed: RightPaneContent,
    pub rx: Option<tokio::sync::mpsc::UnboundedReceiver<RightPaneAsyncReady>>,
}

/// Top-level TUI state. Menu and UI sub-states are grouped into nested structs.
pub struct UblxState {
    pub main_mode: MainMode,
    pub right_pane_mode: RightPaneMode,
    pub panels: PanelState,
    pub search: SearchState,
    pub theme: ThemeState,
    pub toasts: ToastState,
    pub open_menu: OpenMenuState,
    pub lens_menu: LensMenuState,
    pub space_menu: SpaceMenuState,
    pub enhance_policy_menu: EnhancePolicyMenuState,
    pub lens_confirm: LensConfirmState,
    pub chrome: ViewerChrome,
    pub cached_tree: Option<(String, String)>,
    /// Same file row as last tick: reuse viewer text / cover bytes without disk reads.
    pub viewer_disk_cache: Option<ViewerDiskContentCache>,
    /// Viewer: large markdown only — cached styled [`Text`] + viewport slice on scroll.
    pub viewer_text_cache: Option<cache::ViewerTextCacheEntry>,
    /// Viewer: up to [`crate::render::cache::CSV_VIEWER_TEXT_LRU_CAP`] delimiter-table `Text` bodies by path/width/theme/revision.
    pub csv_table_text_lru:
        cache::LruCache<cache::ViewerTableCacheKey, cache::ViewerTextCacheEntry>,
    /// Image category viewer ([`RightPaneContent::viewer_abs_path`] + [`crate::render::viewers::image`]).
    pub viewer_image: ViewerImageState,
    pub last_key_for_double: Option<char>,
    pub snapshot_bg: BackgroundSnapshot,
    pub duplicate_load: DuplicateLoadGate,
    pub config_written_by_us_at: Option<std::time::Instant>,
    pub session: SessionFlow,
    /// CLI to pipe UTF-8 into for clipboard (see [`ClipboardCopyCommand::detect`]); None if nothing found.
    pub clipboard_copy: Option<ClipboardCopyCommand>,
    /// Shown when the per-root DB file under `ubli/` was new this run ([`crate::config::paths::should_show_initial_prompt`]).
    pub startup_prompt: Option<StartupPromptState>,
    pub settings: SettingsPaneState,
    pub right_pane_async: RightPaneAsync,
}

impl Default for UblxState {
    fn default() -> Self {
        Self::new()
    }
}

impl UblxState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            main_mode: MainMode::default(),
            right_pane_mode: RightPaneMode::default(),
            panels: PanelState::new(),
            search: SearchState::default(),
            theme: ThemeState::default(),
            toasts: ToastState::default(),
            open_menu: OpenMenuState::default(),
            lens_menu: LensMenuState::default(),
            space_menu: SpaceMenuState::default(),
            enhance_policy_menu: EnhancePolicyMenuState::default(),
            lens_confirm: LensConfirmState::default(),
            chrome: ViewerChrome::default(),
            cached_tree: None,
            viewer_disk_cache: None,
            viewer_text_cache: None,
            csv_table_text_lru: cache::LruCache::default(),
            viewer_image: ViewerImageState::default(),
            last_key_for_double: None,
            snapshot_bg: BackgroundSnapshot::default(),
            duplicate_load: DuplicateLoadGate::default(),
            config_written_by_us_at: None,
            session: SessionFlow::default(),
            clipboard_copy: ClipboardCopyCommand::detect(),
            startup_prompt: None,
            settings: SettingsPaneState::default(),
            right_pane_async: RightPaneAsync::default(),
        }
    }

    /// Reset open menu state (Esc or after action).
    pub fn close_open_menu(&mut self) {
        self.open_menu.visible = false;
        self.open_menu.path = None;
        self.open_menu.can_terminal = false;
    }

    /// Open the Open (Terminal/GUI) menu. When `can_open_in_terminal` is true, show both options; otherwise only Open (GUI).
    pub fn open_open_menu(&mut self, path: String, can_open_in_terminal: bool) {
        self.open_menu.visible = true;
        self.open_menu.path = Some(path);
        self.open_menu.can_terminal = can_open_in_terminal;
        self.open_menu.selected_index = 0;
    }

    /// Reset lens menu state (Esc or after adding to lens). Does not clear [`LensMenuState::name_input`].
    pub fn close_lens_menu(&mut self) {
        self.lens_menu.visible = false;
        self.lens_menu.path = None;
        self.lens_menu.selected_index = 0;
    }

    /// Reset spacebar context menu state.
    pub fn close_space_menu(&mut self) {
        self.space_menu.visible = false;
        self.space_menu.selected_index = 0;
        self.space_menu.kind = None;
    }

    pub fn close_enhance_policy_menu(&mut self) {
        self.enhance_policy_menu.visible = false;
        self.enhance_policy_menu.path = None;
        self.enhance_policy_menu.selected_index = 0;
    }

    /// Reset delete-lens confirmation popup state.
    pub fn close_lens_delete_confirm(&mut self) {
        self.lens_confirm.delete_visible = false;
        self.lens_confirm.delete_lens_name = None;
        self.lens_confirm.delete_selected = 0;
    }

    /// Open the Lens menu (Add to lens) for the given relative path.
    pub fn open_lens_menu(&mut self, path: String) {
        self.lens_menu.visible = true;
        self.lens_menu.path = Some(path);
        self.lens_menu.selected_index = 0;
    }

    /// Open the spacebar context menu with the given kind.
    pub fn open_space_menu(&mut self, kind: SpaceMenuKind) {
        self.space_menu.visible = true;
        self.space_menu.selected_index = 0;
        self.space_menu.kind = Some(kind);
    }

    /// Show the delete-lens confirmation for the given lens name.
    pub fn open_lens_delete_confirm(&mut self, lens_name: String) {
        self.lens_confirm.delete_visible = true;
        self.lens_confirm.delete_lens_name = Some(lens_name);
        self.lens_confirm.delete_selected = 0;
    }
}

/// Which config file the Settings tab edits (`~/.config/ublx/ublx.toml` vs project `ublx.toml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsConfigScope {
    #[default]
    Global,
    Local,
}

/// Settings tab: bool/layout editor, raw TOML preview scroll, and path to the file being edited.
#[derive(Clone, Debug)]
pub struct SettingsPaneState {
    pub scope: SettingsConfigScope,
    /// Focus row on the left: bool indices, then layout button, then three layout fields when unlocked.
    pub left_cursor: usize,
    pub right_scroll: u16,
    pub layout_unlocked: bool,
    pub layout_left_buf: String,
    pub layout_mid_buf: String,
    pub layout_right_buf: String,
    /// Resolved path for the active scope (refreshed on enter / scope change).
    pub editing_path: Option<std::path::PathBuf>,
}

impl Default for SettingsPaneState {
    fn default() -> Self {
        Self {
            scope: SettingsConfigScope::Global,
            left_cursor: 0,
            right_scroll: 0,
            layout_unlocked: false,
            layout_left_buf: String::new(),
            layout_mid_buf: String::new(),
            layout_right_buf: String::new(),
            editing_path: None,
        }
    }
}

/// Top-level mode: Snapshot, Delta, Settings, Duplicates (if any), or Lenses (if any).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum MainMode {
    #[default]
    Snapshot,
    Delta,
    /// Single-pane config editor (global / local `ublx.toml`).
    Settings,
    Duplicates,
    Lenses,
}

impl MainMode {
    /// Cycle Snapshot → Delta → Settings → Lenses (when available) → Duplicates (when available) → Snapshot. Used for `MainModeToggle` (Shift+Tab).
    #[must_use]
    pub fn next(self, has_duplicates: bool, has_lenses: bool) -> MainMode {
        match self {
            MainMode::Snapshot => MainMode::Delta,
            MainMode::Delta => MainMode::Settings,
            MainMode::Settings if has_lenses => MainMode::Lenses,
            MainMode::Settings | MainMode::Lenses if has_duplicates => MainMode::Duplicates,
            MainMode::Settings | MainMode::Lenses | MainMode::Duplicates => MainMode::Snapshot,
        }
    }
}

/// Which panel has focus (Categories or Contents; Metadata is read-only).
#[derive(Clone, Copy, Default, PartialEq)]
pub enum PanelFocus {
    #[default]
    Categories,
    Contents,
}

/// Which variant of the spacebar context menu is open (determines items and Enter behavior).
#[derive(Clone, Debug)]
pub enum SpaceMenuKind {
    /// File actions for a selected file path (relative): Open, Show in folder, Copy Path,
    /// optional enhance actions, and add/remove lens. `can_open_in_terminal`: when true,
    /// Open shows Terminal+GUI; else GUI only.
    FileActions {
        path: String,
        can_open_in_terminal: bool,
        /// Show subtree batch-enhance policy when the snapshot row is [`CATEGORY_DIRECTORY`].
        show_enhance_directory_policy: bool,
        /// Show "Enhance with `ZahirScan`" when [`crate::config::UblxOpts::enable_enhance_all`] is false and row has no `zahir_json`.
        show_enhance_zahir: bool,
    },
    /// Lens panel actions: `lens_name` is the selected lens. Options: Rename, Delete.
    LensPanelActions { lens_name: String },
}

/// Per-pane content from zahir JSON. Templates always present; metadata and writing only if keys exist.
pub struct SectionedPreview {
    pub templates: String,
    pub metadata: Option<String>,
    pub writing: Option<String>,
}

/// Snapshot mode: indices into the single in-memory list (no copy). Delta mode: small owned vec.
#[derive(Clone)]
pub enum ViewContents {
    /// Indices into the caller's `all_rows` slice (snapshot mode — one copy of list).
    SnapshotIndices(Vec<usize>),
    /// Owned rows for delta mode (added/mod/removed paths; typically small).
    DeltaRows(Vec<TuiRow>),
}

/// Derived list data for this tick: filtered categories and contents (by index or owned), lengths for navigation.
/// Scalability: snapshot mode uses [`ViewContents::SnapshotIndices`] so we keep a single copy of the list; no cloned row vec.
pub struct ViewData {
    pub filtered_categories: Vec<String>,
    pub contents: ViewContents,
    pub category_list_len: usize,
    pub content_len: usize,
}

impl ViewData {
    /// Row at content index `i`. For [`ViewContents::SnapshotIndices`], pass `Some(all_rows)`; for [`ViewContents::DeltaRows`], pass `None`.
    #[must_use]
    pub fn row_at<'a>(&'a self, i: usize, all_rows: Option<&'a [TuiRow]>) -> Option<&'a TuiRow> {
        match &self.contents {
            ViewContents::SnapshotIndices(indices) => indices
                .get(i)
                .and_then(|&pos| all_rows.and_then(|r| r.get(pos))),
            ViewContents::DeltaRows(rows) => rows.get(i),
        }
    }

    /// Iterate over content rows. For [`ViewContents::SnapshotIndices`], pass `Some(all_rows)`; for [`ViewContents::DeltaRows`], pass `None`.
    #[must_use]
    pub fn iter_contents<'a>(
        &'a self,
        all_rows: Option<&'a [TuiRow]>,
    ) -> Box<dyn Iterator<Item = &'a TuiRow> + 'a> {
        match &self.contents {
            ViewContents::SnapshotIndices(indices) => {
                let iter = indices
                    .iter()
                    .filter_map(move |&pos| all_rows.and_then(|r| r.get(pos)));
                Box::new(iter)
            }
            ViewContents::DeltaRows(rows) => Box::new(rows.iter()),
        }
    }
}

/// Raw delta row: (`created_ns`, path) from `delta_log`. Used to build display lines with dates preserved when filtering.
pub type DeltaRow = (i64, String);

/// Data for Delta mode: snapshot overview text and raw (`created_ns`, path) rows per delta type.
pub struct DeltaViewData {
    pub overview_text: String,
    pub added_rows: Vec<DeltaRow>,
    pub mod_rows: Vec<DeltaRow>,
    pub removed_rows: Vec<DeltaRow>,
}

impl DeltaViewData {
    /// Raw rows for the given category index. Uses [`DeltaType::from_index`].
    #[must_use]
    pub fn rows_by_index(&self, idx: usize) -> &[DeltaRow] {
        match DeltaType::from_index(idx) {
            DeltaType::Added => &self.added_rows,
            DeltaType::Mod => &self.mod_rows,
            DeltaType::Removed => &self.removed_rows,
        }
    }
}

/// Result from background right-pane resolve.
#[derive(Debug)]
pub struct RightPaneAsyncReady {
    pub generation: u64,
    pub path: String,
    pub content: RightPaneContent,
    pub disk_cache: Option<ViewerDiskContentCache>,
}

/// Text to show in the right pane for the current selection.
#[derive(Default, Clone, Debug)]
pub struct RightPaneContent {
    pub templates: String,
    pub metadata: Option<String>,
    pub writing: Option<String>,
    pub viewer: Option<String>,
    /// Path of the file being viewed (when viewer shows file content); used for CSV cache keys, etc.
    pub viewer_path: Option<String>,
    /// Absolute path on disk for the selected file (viewer). Used for image preview and open.
    pub viewer_abs_path: Option<PathBuf>,
    /// Parsed zahir [`FileType`] when snapshot `category` matches [`FileType::as_metadata_name`]; drives viewer mode.
    pub viewer_zahir_type: Option<FileType>,
    /// When viewer shows file content, size in bytes from snapshot (for footer display).
    pub viewer_byte_size: Option<u64>,
    /// When viewer shows file content, mtime in ns from snapshot (for footer last-modified).
    pub viewer_mtime_ns: Option<i64>,
    /// When true, the viewed file is non-binary and can be opened (Shift+O: Open Terminal / Open GUI).
    pub viewer_can_open: bool,
    /// Space menu: offer per-file `ZahirScan` when global enhance is off and this row has no enrichment yet.
    pub viewer_offer_enhance_zahir: bool,
    /// Space menu: offer `[[enhance_policy]]` for this path when the row is a Directory in the snapshot.
    pub viewer_offer_enhance_directory_policy: bool,
    /// Embedded cover image bytes (audio tags / EPUB) for raster preview; [`None`] uses normal text/binary viewer.
    pub viewer_embedded_cover_raster: Option<Vec<u8>>,
}

impl RightPaneContent {
    /// Empty right-pane content (e.g. Delta mode has no selection-based viewer).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum RightPaneMode {
    #[default]
    Viewer,
    Templates,
    Metadata,
    Writing,
}
