//! Key bindings for the vanilla TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Actions for the 3-panel TUI (categories, contents, preview).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UblxAction {
    Quit,
    Help,
    /// Switch to Snapshot main tab.
    MainModeSnapshot,
    /// Switch to Delta main tab.
    MainModeDelta,
    /// Alternate between Snapshot and Delta (Shift+Tab).
    MainModeToggle,
    SearchStart,
    SearchChar(char),
    SearchBackspace,
    SearchSubmit,
    /// Esc when search is active (clear search); when inactive, use Quit.
    SearchClear,
    CycleRightPane,
    RightPaneViewer,
    /// Toggle viewer fullscreen (only when on Viewer tab).
    ViewerFullscreenToggle,
    RightPaneTemplates,
    RightPaneMetadata,
    RightPaneWriting,
    ScrollPreviewUp,
    ScrollPreviewDown,
    /// gg: go to top of list (categories or contents).
    ListTop,
    /// G: go to bottom of list.
    ListBottom,
    /// Ctrl+g: scroll preview to top.
    PreviewTop,
    /// Ctrl+G: scroll preview to bottom.
    PreviewBottom,
    MoveUp,
    MoveDown,
    FocusCategories,
    FocusContents,
    Tab,
    /// Run take-snapshot pipeline in background; completion shows in log bumper.
    TakeSnapshot,
    Noop,
}

/// Result of key mapping: action to run and optional "last key" for double-key (e.g. gg).
pub struct KeyActionResult {
    pub action: UblxAction,
    /// Set when key is the first of a possible double (e.g. first `g` for gg); clear on any other key.
    pub last_key_for_double: Option<char>,
}

/// Map a key event to a vanilla TUI action. Call only when `event.kind == KeyEventKind::Press`.
/// Esc yields SearchClear when the search bar is open or when a filter is active (so Esc clears
/// search instead of quitting). Only when not searching at all does Esc mean Quit.
/// Pass `last_key_for_double` from state to detect gg (two g's) for ListTop.
pub fn key_action_setup(
    event: KeyEvent,
    search_active: bool,
    has_search_filter: bool,
    last_key_for_double: Option<char>,
) -> KeyActionResult {
    if event.kind != KeyEventKind::Press {
        return KeyActionResult {
            action: UblxAction::Noop,
            last_key_for_double: None,
        };
    }
    let shift = event.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let (action, last_key) = match event.code {
        KeyCode::Esc if search_active || has_search_filter => (UblxAction::SearchClear, None),
        KeyCode::Char('q') | KeyCode::Esc => (UblxAction::Quit, None),
        KeyCode::Char('?') => (UblxAction::Help, None),
        KeyCode::Char('/') if !search_active => (UblxAction::SearchStart, None),
        KeyCode::Char(c) if search_active => (UblxAction::SearchChar(c), None),
        KeyCode::Char('s' | 'S') if shift => (UblxAction::TakeSnapshot, None),
        KeyCode::Char('f' | 'F') if shift => (UblxAction::ViewerFullscreenToggle, None),
        KeyCode::Char('v' | 'V') if shift => (UblxAction::CycleRightPane, None),
        KeyCode::Char('J') if shift => (UblxAction::ScrollPreviewDown, None),
        KeyCode::Char('K') if shift => (UblxAction::ScrollPreviewUp, None),
        KeyCode::Char('b' | 'B') if ctrl => (UblxAction::PreviewTop, None),
        KeyCode::Char('e' | 'E') if ctrl => (UblxAction::PreviewBottom, None),
        KeyCode::Char('G') if shift => (UblxAction::ListBottom, None),
        KeyCode::Char('g') if !shift && !ctrl => {
            if last_key_for_double == Some('g') {
                (UblxAction::ListTop, None)
            } else {
                (UblxAction::Noop, Some('g'))
            }
        }
        KeyCode::Char(c) if shift => (UblxAction::SearchChar(c), None),
        KeyCode::Char(c) => {
            let a = match c {
                '1' => UblxAction::MainModeSnapshot,
                '2' => UblxAction::MainModeDelta,
                'v' => UblxAction::RightPaneViewer,
                't' => UblxAction::RightPaneTemplates,
                'm' => UblxAction::RightPaneMetadata,
                'w' => UblxAction::RightPaneWriting,
                'j' => UblxAction::MoveDown,
                'k' => UblxAction::MoveUp,
                'h' => UblxAction::FocusCategories,
                'l' => UblxAction::FocusContents,
                'J' => UblxAction::ScrollPreviewDown,
                'K' => UblxAction::ScrollPreviewUp,
                _ => UblxAction::Noop,
            };
            (a, None)
        }
        KeyCode::Enter => (UblxAction::SearchSubmit, None),
        KeyCode::Backspace => (UblxAction::SearchBackspace, None),
        KeyCode::Up if shift => (UblxAction::ScrollPreviewUp, None),
        KeyCode::Down if shift => (UblxAction::ScrollPreviewDown, None),
        KeyCode::Up => (UblxAction::MoveUp, None),
        KeyCode::Down => (UblxAction::MoveDown, None),
        KeyCode::Left => (UblxAction::FocusCategories, None),
        KeyCode::Right => (UblxAction::FocusContents, None),
        KeyCode::Tab => (UblxAction::Tab, None),
        KeyCode::BackTab => (UblxAction::MainModeToggle, None),
        _ => (UblxAction::Noop, None),
    };
    KeyActionResult {
        action,
        last_key_for_double: last_key,
    }
}

/// Returns true if the action was handled by the search bar (main loop should skip navigation).
pub fn search_consumes(action: UblxAction) -> bool {
    matches!(
        action,
        UblxAction::SearchClear
            | UblxAction::SearchSubmit
            | UblxAction::SearchBackspace
            | UblxAction::SearchChar(_)
    )
}
