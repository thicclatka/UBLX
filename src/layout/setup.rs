//! 3-panel TUI: categories (left), contents (middle), preview (right).
//!
//! `run_ublx` is split into four phases per tick (see classification below).

use ratatui::style::Style;
use ratatui::widgets::ListState;

use crate::ui::keymap::UblxAction;

use super::style;

/// Snapshot row for TUI: (path_str, category, zahir_json).
pub type TuiRow = (String, String, String);

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
    pub right_pane_mode: RightPaneMode,
    pub highlight_style: Style,
    /// Set by TakeSnapshot key; event loop spawns pipeline and clears.
    pub snapshot_requested: bool,
    /// When set, show toast until this instant (transient notification).
    pub toast_visible_until: Option<std::time::Instant>,
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
            right_pane_mode: RightPaneMode::default(),
            highlight_style: style::list_highlight(),
            snapshot_requested: false,
            toast_visible_until: None,
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

// -----------------------------------------------------------------------------
// Class 1: View data — filtered lists and selection clamping
// -----------------------------------------------------------------------------

/// Derived list data for this tick: filtered categories/contents and lengths for navigation.
pub struct ViewData {
    pub filtered_categories: Vec<String>,
    pub filtered_contents_rows: Vec<TuiRow>,
    pub category_list_len: usize,
    pub content_len: usize,
}

/// Data for Delta mode: snapshot overview text and paths per delta type (from delta_log).
pub struct DeltaViewData {
    pub overview_text: String,
    pub added_paths: Vec<String>,
    pub mod_paths: Vec<String>,
    pub removed_paths: Vec<String>,
}

// -----------------------------------------------------------------------------
// Class 3: Draw — layout and render all panels
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// Class 4: Input — poll key, map to action, apply to state
// -----------------------------------------------------------------------------

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
            UblxAction::Quit => return true,
            UblxAction::Help => state.help_visible = true,
            UblxAction::MainModeSnapshot => state.main_mode = MainMode::Snapshot,
            UblxAction::MainModeDelta => state.main_mode = MainMode::Delta,
            UblxAction::SearchStart => state.search_active = true,
            UblxAction::CycleRightPane => {
                let available: Vec<RightPaneMode> = [
                    RightPaneMode::Templates,
                    RightPaneMode::Viewer,
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
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum RightPaneMode {
    Viewer,
    #[default]
    Templates,
    Metadata,
    Writing,
}
