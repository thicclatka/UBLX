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
    SearchStart,
    SearchChar(char),
    SearchBackspace,
    SearchSubmit,
    /// Esc when search is active (clear search); when inactive, use Quit.
    SearchClear,
    CycleRightPane,
    RightPaneViewer,
    RightPaneTemplates,
    RightPaneMetadata,
    RightPaneWriting,
    ScrollPreviewUp,
    ScrollPreviewDown,
    MoveUp,
    MoveDown,
    FocusCategories,
    FocusContents,
    Tab,
    /// Run take-snapshot pipeline in background; completion shows in log bumper.
    TakeSnapshot,
    Noop,
}

/// Map a key event to a vanilla TUI action. Call only when `event.kind == KeyEventKind::Press`.
/// Esc yields SearchClear when the search bar is open or when a filter is active (so Esc clears
/// search instead of quitting). Only when not searching at all does Esc mean Quit.
pub fn key_action_setup(
    event: KeyEvent,
    search_active: bool,
    has_search_filter: bool,
) -> UblxAction {
    if event.kind != KeyEventKind::Press {
        return UblxAction::Noop;
    }
    let shift = event.modifiers.contains(KeyModifiers::SHIFT);
    match event.code {
        KeyCode::Esc if search_active || has_search_filter => UblxAction::SearchClear,
        KeyCode::Char('q') | KeyCode::Esc => UblxAction::Quit,
        KeyCode::Char('?') => UblxAction::Help,
        KeyCode::Char('/') if !search_active => UblxAction::SearchStart,
        KeyCode::Char(c) if search_active => UblxAction::SearchChar(c),
        KeyCode::Char('s' | 'S') if shift => UblxAction::TakeSnapshot,
        KeyCode::Char(c) if shift => UblxAction::SearchChar(c),
        KeyCode::Char(c) => match c {
            '1' => UblxAction::MainModeSnapshot,
            '2' => UblxAction::MainModeDelta,
            'V' => UblxAction::CycleRightPane,
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
        },
        KeyCode::Enter => UblxAction::SearchSubmit,
        KeyCode::Backspace => UblxAction::SearchBackspace,
        KeyCode::Up if shift => UblxAction::ScrollPreviewUp,
        KeyCode::Down if shift => UblxAction::ScrollPreviewDown,
        KeyCode::Up => UblxAction::MoveUp,
        KeyCode::Down => UblxAction::MoveDown,
        KeyCode::Left => UblxAction::FocusCategories,
        KeyCode::Right => UblxAction::FocusContents,
        KeyCode::Tab => UblxAction::Tab,
        _ => UblxAction::Noop,
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
