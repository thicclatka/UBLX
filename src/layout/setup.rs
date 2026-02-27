//! 3-panel TUI: categories (left), contents (middle), preview (right).
//!
//! `run_ublx` is split into four phases per tick (see classification below).
//! Action application (key → state changes) lives in [crate::handlers::state_transitions].

use std::collections::HashMap;

use ratatui::style::Style;
use ratatui::widgets::ListState;

use super::style;

use crate::engine::db_ops::DeltaType;
/// Row for TUI list: (path, category, size_bytes). Same as [crate::engine::db_ops::SnapshotTuiRow]; zahir_json is loaded on demand for the selected row.
pub use crate::engine::db_ops::SnapshotTuiRow as TuiRow;

/// Category string for directories in the snapshot (matches [crate::engine::db_ops::UblxDbCategory]).
pub const CATEGORY_DIRECTORY: &str = "Directory";

pub struct UblxState {
    pub main_mode: MainMode,
    pub focus: PanelFocus,
    pub category_state: ListState,
    pub content_state: ListState,
    pub preview_scroll: u16,
    pub prev_preview_key: Option<(usize, Option<usize>)>,
    pub search_query: String,
    pub search_active: bool,
    pub cached_tree: Option<(String, String)>,
    pub help_visible: bool,
    /// When true, show theme selector popup; j/k to move, Enter to pick and save, Esc to revert.
    pub theme_selector_visible: bool,
    /// Selected index in theme_options() when theme selector is open.
    pub theme_selector_index: usize,
    /// Theme name before opening selector; restored on Esc.
    pub theme_before_selector: Option<String>,
    /// Override theme for this run (set when user picks in selector; used instead of opts theme).
    pub theme_override: Option<String>,
    pub right_pane_mode: RightPaneMode,
    pub highlight_style: Style,
    /// Set by TakeSnapshot key; event loop spawns pipeline and clears.
    pub snapshot_requested: bool,
    /// Stack of toasts (each has its own timer); oldest first, newest last.
    pub toast_slots: Vec<crate::utils::notifications::ToastSlot>,
    /// Per-operation count of messages we've already shown in a toast (so the next toast only shows new ones).
    pub toast_consumed_per_operation: HashMap<String, usize>,
    /// Viewer takes full screen (hide categories and contents).
    pub viewer_fullscreen: bool,
    /// For double-key detection (e.g. gg → ListTop). Cleared on any other key.
    pub last_key_for_double: Option<char>,
    /// When set, we poll the snapshot DB (e.g. .ublx_tmp) at most when this time is reached.
    pub snapshot_poll_deadline: Option<std::time::Instant>,
    /// True after we've received a "snapshot done" message; reset when user triggers a new snapshot so we poll again.
    pub snapshot_done_received: bool,
    /// Set by Ctrl+D; event loop spawns duplicate detection and clears this.
    pub duplicate_load_requested: bool,
    /// When set, we recently wrote the config ourselves (e.g. theme selector). Used to avoid showing "Config reload (triggered by save)" for our own write.
    pub config_written_by_us_at: Option<std::time::Instant>,
    /// True on first tick only; used to show any ublx-settings bumper messages (e.g. "config invalid at startup, using cache") as a toast.
    pub first_tick: bool,
    /// When true, show Open (Terminal) / Open (GUI) popup below selection. Path is relative to indexed dir.
    pub open_menu_visible: bool,
    /// Path of file to open (relative) when open_menu_visible. None when menu closed.
    pub open_menu_path: Option<String>,
    /// Selected index in open menu: 0 = Open (Terminal), 1 = Open (GUI).
    pub open_menu_selected_index: usize,
    /// When true, the next tick should re-apply terminal state and redraw (set after Open (Terminal) returns so the TUI repaints correctly).
    pub refresh_terminal_after_editor: bool,
}

impl Default for UblxState {
    fn default() -> Self {
        Self::new()
    }
}

impl UblxState {
    pub fn new() -> Self {
        let mut state = Self {
            main_mode: MainMode::default(),
            focus: PanelFocus::default(),
            category_state: ListState::default(),
            content_state: ListState::default(),
            preview_scroll: 0,
            prev_preview_key: None,
            search_query: String::new(),
            search_active: false,
            cached_tree: None,
            help_visible: false,
            theme_selector_visible: false,
            theme_selector_index: 0,
            theme_before_selector: None,
            theme_override: None,
            right_pane_mode: RightPaneMode::default(),
            highlight_style: style::list_highlight(),
            snapshot_requested: false,
            toast_slots: Vec::new(),
            toast_consumed_per_operation: HashMap::new(),
            viewer_fullscreen: false,
            last_key_for_double: None,
            snapshot_poll_deadline: None,
            snapshot_done_received: false, // poll until we receive done; run_ublx sets true when initial load has data (already-done dir)
            duplicate_load_requested: false,
            config_written_by_us_at: None,
            first_tick: true,
            open_menu_visible: false,
            open_menu_path: None,
            open_menu_selected_index: 0,
            refresh_terminal_after_editor: false,
        };
        state.category_state.select(Some(0));
        state.content_state.select(Some(0));
        state
    }
}

/// Top-level mode: Snapshot (categories/contents/preview), Delta (added/mod/removed), or Duplicates (only if any exist).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum MainMode {
    #[default]
    Snapshot,
    Delta,
    Duplicates,
}

impl MainMode {
    /// Cycle Snapshot → Delta → Duplicates (when available) → Snapshot. Used for MainModeToggle (Shift+Tab).
    pub fn next(self, has_duplicates: bool) -> MainMode {
        match self {
            MainMode::Snapshot => MainMode::Delta,
            MainMode::Delta if has_duplicates => MainMode::Duplicates,
            MainMode::Delta | MainMode::Duplicates => MainMode::Snapshot,
        }
    }
}

/// Which panel has focus (Categories or Contents; Metadata is read-only).
#[derive(Clone, Copy, Default)]
pub enum PanelFocus {
    #[default]
    Categories,
    Contents,
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
    /// Indices into the caller's all_rows slice (snapshot mode — one copy of list).
    SnapshotIndices(Vec<usize>),
    /// Owned rows for delta mode (added/mod/removed paths; typically small).
    DeltaRows(Vec<TuiRow>),
}

/// Derived list data for this tick: filtered categories and contents (by index or owned), lengths for navigation.
/// Scalability: snapshot mode uses [ViewContents::SnapshotIndices] so we keep a single copy of the list; no cloned row vec.
pub struct ViewData {
    pub filtered_categories: Vec<String>,
    pub contents: ViewContents,
    pub category_list_len: usize,
    pub content_len: usize,
}

impl ViewData {
    /// Row at content index `i`. For [ViewContents::SnapshotIndices], pass `Some(all_rows)`; for [ViewContents::DeltaRows], pass `None`.
    pub fn row_at<'a>(&'a self, i: usize, all_rows: Option<&'a [TuiRow]>) -> Option<&'a TuiRow> {
        match &self.contents {
            ViewContents::SnapshotIndices(indices) => indices
                .get(i)
                .and_then(|&pos| all_rows.and_then(|r| r.get(pos))),
            ViewContents::DeltaRows(rows) => rows.get(i),
        }
    }

    /// Iterate over content rows. For [ViewContents::SnapshotIndices], pass `Some(all_rows)`; for [ViewContents::DeltaRows], pass `None`.
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

/// Raw delta row: (created_ns, path) from delta_log. Used to build display lines with dates preserved when filtering.
pub type DeltaRow = (i64, String);

/// Data for Delta mode: snapshot overview text and raw (created_ns, path) rows per delta type.
pub struct DeltaViewData {
    pub overview_text: String,
    pub added_rows: Vec<DeltaRow>,
    pub mod_rows: Vec<DeltaRow>,
    pub removed_rows: Vec<DeltaRow>,
}

impl DeltaViewData {
    /// Raw rows for the given category index. Uses [DeltaType::from_index].
    pub fn rows_by_index(&self, idx: usize) -> &[DeltaRow] {
        match DeltaType::from_index(idx) {
            DeltaType::Added => &self.added_rows,
            DeltaType::Mod => &self.mod_rows,
            DeltaType::Removed => &self.removed_rows,
        }
    }
}
/// Text to show in the right pane for the current selection.
pub struct RightPaneContent {
    pub templates: String,
    pub metadata: Option<String>,
    pub writing: Option<String>,
    pub viewer: Option<String>,
    /// Path of the file being viewed (when viewer shows file content), for markdown detection.
    pub viewer_path: Option<String>,
    /// When viewer shows file content, size in bytes from snapshot (for footer display).
    pub viewer_byte_size: Option<u64>,
    /// When viewer shows file content, mtime in ns from snapshot (for footer last-modified).
    pub viewer_mtime_ns: Option<i64>,
    /// When true, the viewed file is non-binary and can be opened (Shift+O: Open Terminal / Open GUI).
    pub viewer_can_open: bool,
    /// Label for the open hint node in the footer (e.g. "↗", "↗ (Terminal)", "↗ (GUI)"). Set by caller when viewer_can_open.
    pub open_hint_label: Option<String>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum RightPaneMode {
    #[default]
    Viewer,
    Templates,
    Metadata,
    Writing,
}
