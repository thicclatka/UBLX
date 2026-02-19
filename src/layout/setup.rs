//! 3-panel TUI: categories (left), contents (middle), preview (right).
//!
//! `run_ublx` is split into four phases per tick (see classification below).

use ratatui::style::Style;
use ratatui::widgets::ListState;

use crate::ui::keymap::UblxAction;

use super::style;

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
    /// Viewer takes full screen (hide categories and contents).
    pub viewer_fullscreen: bool,
    /// For double-key detection (e.g. gg → ListTop). Cleared on any other key.
    pub last_key_for_double: Option<char>,
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
            viewer_fullscreen: false,
            last_key_for_double: None,
        };
        state.category_state.select(Some(0));
        state.content_state.select(Some(0));
        state
    }
}

/// Top-level mode: Snapshot (categories/contents/preview) or Delta (added/mod/removed + overview).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum MainMode {
    #[default]
    Snapshot,
    Delta,
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
    /// Raw rows for the given category index: 0 = added, 1 = mod, 2 = removed.
    pub fn rows_by_index(&self, idx: usize) -> &[DeltaRow] {
        match idx {
            0 => &self.added_rows,
            1 => &self.mod_rows,
            _ => &self.removed_rows,
        }
    }
}
/// Context (view + right-pane content) required to apply actions to state.
pub struct UblxActionContext<'a> {
    view: &'a ViewData,
    right: &'a RightPaneContent,
}

impl<'a> UblxActionContext<'a> {
    pub fn new(view: &'a ViewData, right: &'a RightPaneContent) -> Self {
        Self { view, right }
    }

    /// Apply the key action to state (mutates focus, selection, panes, etc.).
    /// Returns true if the user requested quit (caller should exit the run loop).
    pub fn apply_action_to_state(&self, state: &mut UblxState, action: UblxAction) -> bool {
        match action {
            UblxAction::Quit => {
                if state.viewer_fullscreen {
                    state.viewer_fullscreen = false;
                } else {
                    return true;
                }
            }
            UblxAction::Help => state.help_visible = true,
            UblxAction::MainModeSnapshot => state.main_mode = MainMode::Snapshot,
            UblxAction::MainModeDelta => state.main_mode = MainMode::Delta,
            UblxAction::MainModeToggle => {
                state.main_mode = match state.main_mode {
                    MainMode::Snapshot => MainMode::Delta,
                    MainMode::Delta => MainMode::Snapshot,
                };
            }
            UblxAction::SearchStart => state.search_active = true,
            UblxAction::CycleRightPane => {
                let available: Vec<RightPaneMode> = [
                    RightPaneMode::Viewer,
                    RightPaneMode::Templates,
                    RightPaneMode::Metadata,
                    RightPaneMode::Writing,
                ]
                .into_iter()
                .filter(|m| match m {
                    RightPaneMode::Templates | RightPaneMode::Viewer => true,
                    RightPaneMode::Metadata => self.right.metadata.is_some(),
                    RightPaneMode::Writing => self.right.writing.is_some(),
                })
                .collect();
                if !available.is_empty() {
                    let idx = available
                        .iter()
                        .position(|m| *m == state.right_pane_mode)
                        .unwrap_or(0);
                    let next = (idx + 1) % available.len();
                    state.right_pane_mode = available[next];
                }
            }
            UblxAction::RightPaneViewer => state.right_pane_mode = RightPaneMode::Viewer,
            UblxAction::ViewerFullscreenToggle => {
                if state.right_pane_mode == RightPaneMode::Viewer {
                    state.viewer_fullscreen = !state.viewer_fullscreen;
                }
            }
            UblxAction::RightPaneTemplates => state.right_pane_mode = RightPaneMode::Templates,
            UblxAction::RightPaneMetadata => {
                if self.right.metadata.is_some() {
                    state.right_pane_mode = RightPaneMode::Metadata;
                }
            }
            UblxAction::RightPaneWriting => {
                if self.right.writing.is_some() {
                    state.right_pane_mode = RightPaneMode::Writing;
                }
            }
            UblxAction::ScrollPreviewUp => {
                state.preview_scroll = state.preview_scroll.saturating_sub(1);
            }
            UblxAction::ScrollPreviewDown => {
                state.preview_scroll = state.preview_scroll.saturating_add(1);
            }
            UblxAction::ListTop => match state.focus {
                PanelFocus::Categories => {
                    if self.view.category_list_len > 0 {
                        state.category_state.select(Some(0));
                    }
                }
                PanelFocus::Contents => {
                    if self.view.content_len > 0 {
                        state.content_state.select(Some(0));
                    }
                }
            },
            UblxAction::ListBottom => match state.focus {
                PanelFocus::Categories => {
                    if self.view.category_list_len > 0 {
                        let last = self.view.category_list_len.saturating_sub(1);
                        state.category_state.select(Some(last));
                    }
                }
                PanelFocus::Contents => {
                    if self.view.content_len > 0 {
                        let last = self.view.content_len.saturating_sub(1);
                        state.content_state.select(Some(last));
                    }
                }
            },
            UblxAction::PreviewTop => state.preview_scroll = 0,
            UblxAction::PreviewBottom => state.preview_scroll = u16::MAX,
            UblxAction::MoveUp => match state.focus {
                PanelFocus::Categories => {
                    if self.view.category_list_len > 0 {
                        let i = state.category_state.selected().unwrap_or(0);
                        state.category_state.select(Some(
                            i.saturating_sub(1)
                                .min(self.view.category_list_len.saturating_sub(1)),
                        ));
                    }
                }
                PanelFocus::Contents => {
                    if self.view.content_len > 0 {
                        let i = state.content_state.selected().unwrap_or(0);
                        state.content_state.select(Some(
                            i.saturating_sub(1)
                                .min(self.view.content_len.saturating_sub(1)),
                        ));
                    }
                }
            },
            UblxAction::MoveDown => match state.focus {
                PanelFocus::Categories => {
                    if self.view.category_list_len > 0 {
                        let i = state.category_state.selected().unwrap_or(0);
                        state.category_state.select(Some(
                            (i + 1).min(self.view.category_list_len.saturating_sub(1)),
                        ));
                    }
                }
                PanelFocus::Contents => {
                    if self.view.content_len > 0 {
                        let i = state.content_state.selected().unwrap_or(0);
                        state
                            .content_state
                            .select(Some((i + 1).min(self.view.content_len.saturating_sub(1))));
                    }
                }
            },
            UblxAction::FocusCategories => state.focus = PanelFocus::Categories,
            UblxAction::FocusContents => state.focus = PanelFocus::Contents,
            UblxAction::Tab => {
                state.focus = match state.focus {
                    PanelFocus::Categories => PanelFocus::Contents,
                    PanelFocus::Contents => PanelFocus::Categories,
                };
            }
            UblxAction::TakeSnapshot => state.snapshot_requested = true,
            _ => {}
        }
        false
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
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum RightPaneMode {
    #[default]
    Viewer,
    Templates,
    Metadata,
    Writing,
}
