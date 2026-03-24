//! Apply key actions to TUI state. Moved from layout so "what happens on key" lives with other behavior.

use crate::handlers::zahir_ops::ZahirFileType as FileType;
use crate::layout::setup::{
    MainMode, PanelFocus, RightPaneContent, RightPaneMode, UblxState, ViewData,
};
use crate::ui::keymap::UblxAction;
use crate::utils::clamp_selection;

fn apply_quit(state: &mut UblxState) -> bool {
    if state.chrome.viewer_fullscreen {
        state.chrome.viewer_fullscreen = false;
        false
    } else {
        true
    }
}

fn apply_misc(state: &mut UblxState, action: UblxAction) {
    match action {
        UblxAction::Help => state.chrome.help_visible = true,
        UblxAction::TakeSnapshot => state.snapshot_bg.requested = true,
        _ => {}
    }
}

fn apply_mode_switch(
    state: &mut UblxState,
    action: UblxAction,
    has_duplicates: bool,
    has_lenses: bool,
) {
    match action {
        UblxAction::MainModeSnapshot => state.main_mode = MainMode::Snapshot,
        UblxAction::MainModeDelta => state.main_mode = MainMode::Delta,
        UblxAction::MainModeDuplicates => state.main_mode = MainMode::Duplicates,
        UblxAction::MainModeLenses => state.main_mode = MainMode::Lenses,
        UblxAction::LoadDuplicates => state.duplicate_load.requested = true,
        UblxAction::MainModeToggle => {
            state.main_mode = state.main_mode.next(has_duplicates, has_lenses);
        }
        _ => {}
    }
}

fn apply_preview_scroll(state: &mut UblxState, action: UblxAction) {
    match action {
        UblxAction::ScrollPreviewUp => {
            state.panels.preview_scroll = state.panels.preview_scroll.saturating_sub(1);
        }
        UblxAction::ScrollPreviewDown => {
            state.panels.preview_scroll = state.panels.preview_scroll.saturating_add(1);
        }
        UblxAction::PreviewTop => state.panels.preview_scroll = 0,
        UblxAction::PreviewBottom => state.panels.preview_scroll = u16::MAX,
        _ => {}
    }
}

fn pdf_page_nav_applies(state: &UblxState, right: &RightPaneContent) -> bool {
    state.right_pane_mode == RightPaneMode::Viewer
        && right.viewer_zahir_type == Some(FileType::Pdf)
        && right.viewer_abs_path.is_some()
}

fn apply_pdf_page_scroll(state: &mut UblxState, action: UblxAction) {
    let max = state.viewer_image.pdf_page_count;
    match action {
        UblxAction::ScrollPreviewDown => {
            let next = state.viewer_image.pdf_page.saturating_add(1);
            state.viewer_image.pdf_page = if let Some(m) = max { next.min(m) } else { next };
        }
        UblxAction::ScrollPreviewUp => {
            state.viewer_image.pdf_page = state.viewer_image.pdf_page.saturating_sub(1).max(1);
        }
        UblxAction::PreviewTop => {
            state.viewer_image.pdf_page = 1;
        }
        UblxAction::PreviewBottom => {
            if let Some(m) = max {
                state.viewer_image.pdf_page = m;
            }
        }
        _ => {}
    }
}

/// Context (view + right-pane content) required to apply actions to state.
pub struct UblxActionContext<'a> {
    view: &'a ViewData,
    right_content: &'a RightPaneContent,
}

impl<'a> UblxActionContext<'a> {
    #[must_use]
    pub fn new(view: &'a ViewData, right_content: &'a RightPaneContent) -> Self {
        Self {
            view,
            right_content,
        }
    }

    /// Apply the key action to state (mutates focus, selection, panes, etc.).
    /// Returns true if the user requested quit (caller should exit the run loop).
    /// `has_duplicates` / `has_lenses` are used for `MainModeToggle` and tab keys (cycle / switch only when tab exists).
    pub fn apply_action_to_state(
        &self,
        state: &mut UblxState,
        action: UblxAction,
        has_duplicates: bool,
        has_lenses: bool,
    ) -> bool {
        if let UblxAction::Quit = action {
            return apply_quit(state);
        }
        match action {
            UblxAction::Help | UblxAction::TakeSnapshot => apply_misc(state, action),
            UblxAction::MainModeSnapshot
            | UblxAction::MainModeDelta
            | UblxAction::MainModeDuplicates
            | UblxAction::MainModeLenses
            | UblxAction::MainModeToggle
            | UblxAction::LoadDuplicates => {
                apply_mode_switch(state, action, has_duplicates, has_lenses);
            }
            UblxAction::SearchStart => state.search.active = true,
            UblxAction::CycleRightPane
            | UblxAction::RightPaneViewer
            | UblxAction::ViewerFullscreenToggle
            | UblxAction::RightPaneTemplates
            | UblxAction::RightPaneMetadata
            | UblxAction::RightPaneWriting => self.apply_right_pane(state, action),
            UblxAction::ScrollPreviewUp
            | UblxAction::ScrollPreviewDown
            | UblxAction::PreviewTop
            | UblxAction::PreviewBottom => {
                if pdf_page_nav_applies(state, self.right_content) {
                    apply_pdf_page_scroll(state, action);
                } else {
                    apply_preview_scroll(state, action);
                }
            }
            UblxAction::ListTop
            | UblxAction::ListBottom
            | UblxAction::MoveUp
            | UblxAction::MoveDown
            | UblxAction::FocusCategories
            | UblxAction::FocusContents
            | UblxAction::Tab => self.apply_navigation(state, action),
            _ => {}
        }
        false
    }

    fn apply_right_pane(&self, state: &mut UblxState, action: UblxAction) {
        match action {
            UblxAction::CycleRightPane => self.apply_cycle_right_pane(state),
            UblxAction::RightPaneViewer => state.right_pane_mode = RightPaneMode::Viewer,
            UblxAction::ViewerFullscreenToggle => {
                state.chrome.viewer_fullscreen = !state.chrome.viewer_fullscreen;
            }
            UblxAction::RightPaneTemplates => {
                if !self.right_content.templates.is_empty() {
                    state.right_pane_mode = RightPaneMode::Templates;
                }
            }
            UblxAction::RightPaneMetadata => {
                if self.right_content.metadata.is_some() {
                    state.right_pane_mode = RightPaneMode::Metadata;
                }
            }
            UblxAction::RightPaneWriting => {
                if self.right_content.writing.is_some() {
                    state.right_pane_mode = RightPaneMode::Writing;
                }
            }
            _ => {}
        }
    }

    fn apply_navigation(&self, state: &mut UblxState, action: UblxAction) {
        match action {
            UblxAction::ListTop => self.apply_list_top(state),
            UblxAction::ListBottom => self.apply_list_bottom(state),
            UblxAction::MoveUp => self.apply_move_up(state),
            UblxAction::MoveDown => self.apply_move_down(state),
            UblxAction::FocusCategories => state.panels.focus = PanelFocus::Categories,
            UblxAction::FocusContents => state.panels.focus = PanelFocus::Contents,
            UblxAction::Tab => {
                state.panels.focus = match state.panels.focus {
                    PanelFocus::Categories => PanelFocus::Contents,
                    PanelFocus::Contents => PanelFocus::Categories,
                };
            }
            _ => {}
        }
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
            RightPaneMode::Templates => !self.right_content.templates.is_empty(),
            RightPaneMode::Metadata => self.right_content.metadata.is_some(),
            RightPaneMode::Writing => self.right_content.writing.is_some(),
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
        match state.panels.focus {
            PanelFocus::Categories => {
                if self.view.category_list_len > 0 {
                    state.panels.category_state.select(Some(0));
                }
            }
            PanelFocus::Contents => {
                if self.view.content_len > 0 {
                    state.panels.content_state.select(Some(0));
                }
            }
        }
    }

    fn apply_list_bottom(&self, state: &mut UblxState) {
        match state.panels.focus {
            PanelFocus::Categories => {
                if self.view.category_list_len > 0 {
                    let last =
                        clamp_selection(self.view.category_list_len, self.view.category_list_len);
                    state.panels.category_state.select(Some(last));
                }
            }
            PanelFocus::Contents => {
                if self.view.content_len > 0 {
                    let last = clamp_selection(self.view.content_len, self.view.content_len);
                    state.panels.content_state.select(Some(last));
                }
            }
        }
    }

    fn apply_move_up(&self, state: &mut UblxState) {
        match state.panels.focus {
            PanelFocus::Categories => {
                if self.view.category_list_len > 0 {
                    let i = state.panels.category_state.selected().unwrap_or(0);
                    state.panels.category_state.select(Some(clamp_selection(
                        i.saturating_sub(1),
                        self.view.category_list_len,
                    )));
                }
            }
            PanelFocus::Contents => {
                if self.view.content_len > 0 {
                    let i = state.panels.content_state.selected().unwrap_or(0);
                    state.panels.content_state.select(Some(clamp_selection(
                        i.saturating_sub(1),
                        self.view.content_len,
                    )));
                }
            }
        }
    }

    fn apply_move_down(&self, state: &mut UblxState) {
        match state.panels.focus {
            PanelFocus::Categories => {
                if self.view.category_list_len > 0 {
                    let i = state.panels.category_state.selected().unwrap_or(0);
                    state
                        .panels
                        .category_state
                        .select(Some(clamp_selection(i + 1, self.view.category_list_len)));
                }
            }
            PanelFocus::Contents => {
                if self.view.content_len > 0 {
                    let i = state.panels.content_state.selected().unwrap_or(0);
                    state
                        .panels
                        .content_state
                        .select(Some(clamp_selection(i + 1, self.view.content_len)));
                }
            }
        }
    }
}
