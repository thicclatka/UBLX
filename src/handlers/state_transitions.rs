//! Apply key actions to TUI state. Moved from layout so "what happens on key" lives with other behavior.

use crate::layout::setup::{
    MainMode, PanelFocus, RightPaneContent, RightPaneMode, UblxState, ViewData,
};
use crate::ui::keymap::UblxAction;

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
            UblxAction::CycleRightPane => self.apply_cycle_right_pane(state),
            UblxAction::RightPaneViewer => state.right_pane_mode = RightPaneMode::Viewer,
            UblxAction::ViewerFullscreenToggle => {
                state.viewer_fullscreen = !state.viewer_fullscreen;
            }
            UblxAction::RightPaneTemplates => {
                if !self.right.templates.is_empty() {
                    state.right_pane_mode = RightPaneMode::Templates;
                }
            }
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
            UblxAction::ListTop => self.apply_list_top(state),
            UblxAction::ListBottom => self.apply_list_bottom(state),
            UblxAction::PreviewTop => state.preview_scroll = 0,
            UblxAction::PreviewBottom => state.preview_scroll = u16::MAX,
            UblxAction::MoveUp => self.apply_move_up(state),
            UblxAction::MoveDown => self.apply_move_down(state),
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

    fn apply_cycle_right_pane(&self, state: &mut UblxState) {
        let available: Vec<RightPaneMode> = [
            RightPaneMode::Viewer,
            RightPaneMode::Templates,
            RightPaneMode::Metadata,
            RightPaneMode::Writing,
        ]
        .into_iter()
        .filter(|m| match m {
            RightPaneMode::Viewer => true,
            RightPaneMode::Templates => !self.right.templates.is_empty(),
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

    fn apply_list_top(&self, state: &mut UblxState) {
        match state.focus {
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
        }
    }

    fn apply_list_bottom(&self, state: &mut UblxState) {
        match state.focus {
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
        }
    }

    fn apply_move_up(&self, state: &mut UblxState) {
        match state.focus {
            PanelFocus::Categories => {
                if self.view.category_list_len > 0 {
                    let i = state.category_state.selected().unwrap_or(0);
                    let next = i
                        .saturating_sub(1)
                        .min(self.view.category_list_len.saturating_sub(1));
                    state.category_state.select(Some(next));
                }
            }
            PanelFocus::Contents => {
                if self.view.content_len > 0 {
                    let i = state.content_state.selected().unwrap_or(0);
                    let next = i
                        .saturating_sub(1)
                        .min(self.view.content_len.saturating_sub(1));
                    state.content_state.select(Some(next));
                }
            }
        }
    }

    fn apply_move_down(&self, state: &mut UblxState) {
        match state.focus {
            PanelFocus::Categories => {
                if self.view.category_list_len > 0 {
                    let i = state.category_state.selected().unwrap_or(0);
                    let next = (i + 1).min(self.view.category_list_len.saturating_sub(1));
                    state.category_state.select(Some(next));
                }
            }
            PanelFocus::Contents => {
                if self.view.content_len > 0 {
                    let i = state.content_state.selected().unwrap_or(0);
                    let next = (i + 1).min(self.view.content_len.saturating_sub(1));
                    state.content_state.select(Some(next));
                }
            }
        }
    }
}
