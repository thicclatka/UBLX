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
    /// Switch to Duplicates main tab (only when duplicates exist).
    MainModeDuplicates,
    /// Switch to Lenses main tab (only when any lenses exist).
    MainModeLenses,
    /// Alternate between Snapshot, Delta, Duplicates, and Lenses when available (Shift+Tab).
    MainModeToggle,
    /// Run duplicate detection in background and show Duplicates tab (Ctrl+D).
    LoadDuplicates,
    SearchStart,
    SearchChar(char),
    SearchBackspace,
    SearchSubmit,
    /// Esc when search is active (clear search); when inactive, use Quit.
    SearchClear,
    /// Cycle right pane tab (Ctrl+V).
    CycleRightPane,
    RightPaneViewer,
    /// Toggle right-pane fullscreen (current tab).
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
    /// Run take-snapshot pipeline in background; completion shows in log bumper (Ctrl+S).
    TakeSnapshot,
    /// Open theme selector popup (j/k to preview, Enter to pick and save to local .ublx.toml, Esc to revert).
    ThemeSelector,
    /// Reload hot-reloadable config (theme, transparent, layout, hash, `show_hidden`) from disk. Ctrl+R.
    ReloadConfig,
    /// Open menu (Shift+O): Open (Terminal) or Open (GUI). Only when selection is a non-binary file.
    OpenMenu,
    /// Lens menu (Ctrl+L): Add current file to a lens or create new lens.
    LensMenu,
    /// Spacebar context menu
    SpaceMenu,
    Noop,
}

/// Result of key mapping: action to run and optional "last key" for double-key (e.g. gg).
pub struct KeyActionResult {
    pub action: UblxAction,
    /// Set when key is the first of a possible double (e.g. first `g` for gg); clear on any other key.
    pub last_key_for_double: Option<char>,
}

/// Search bar state for key resolution (`/` open, typed query, Esc behavior).
#[derive(Clone, Copy, Debug)]
pub struct KeySearchState {
    pub active: bool,
    pub has_filter: bool,
}

/// Whether optional main tabs (Duplicates, Lenses) exist for digit keys and toggle.
#[derive(Clone, Copy, Debug)]
pub struct KeyOptionalTabs {
    pub duplicates: bool,
    pub lenses: bool,
}

/// UI snapshot needed to resolve keys: search mode, filter, gg tracking, which optional tabs exist.
#[derive(Clone, Copy, Debug)]
pub struct KeyActionContext {
    pub search: KeySearchState,
    pub last_key_for_double: Option<char>,
    pub tabs: KeyOptionalTabs,
}

/// Map a key event to a vanilla TUI action. Call only when `event.kind == KeyEventKind::Press`.
/// Esc yields `SearchClear` when the search bar is open or when a filter is active (so Esc clears
/// search instead of quitting). Only when not searching at all does Esc mean Quit.
/// Use `last_key_for_double` in `ctx` to detect gg (two g's) for `ListTop`.
/// Use `tabs.duplicates` / `tabs.lenses` so keys 3/9 and `MainModeToggle` switch only when the tab exists.
#[must_use]
pub fn key_action_setup(event: KeyEvent, ctx: &KeyActionContext) -> KeyActionResult {
    if event.kind != KeyEventKind::Press {
        return KeyActionResult {
            action: UblxAction::Noop,
            last_key_for_double: None,
        };
    }
    let shift = event.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    let (action, last_key) = match event.code {
        KeyCode::Esc if ctx.search.active || ctx.search.has_filter => {
            (UblxAction::SearchClear, None)
        }
        KeyCode::Char('q') | KeyCode::Esc => (UblxAction::Quit, None),
        KeyCode::Char('?') => (UblxAction::Help, None),
        KeyCode::Char('/') if !ctx.search.active => (UblxAction::SearchStart, None),
        KeyCode::Char(c) if ctx.search.active => (UblxAction::SearchChar(c), None),
        KeyCode::Char('f' | 'F') if shift => (UblxAction::ViewerFullscreenToggle, None),
        KeyCode::Char('o' | 'O') if shift => (UblxAction::OpenMenu, None),
        KeyCode::Char('t' | 'T') if ctrl => (UblxAction::ThemeSelector, None),
        KeyCode::Char('J') | KeyCode::Down if shift => (UblxAction::ScrollPreviewDown, None),
        KeyCode::Char('K') | KeyCode::Up if shift => (UblxAction::ScrollPreviewUp, None),
        KeyCode::Char('b' | 'B') if ctrl => (UblxAction::PreviewTop, None),
        KeyCode::Char('d' | 'D') if ctrl => (UblxAction::LoadDuplicates, None),
        KeyCode::Char('e' | 'E') if ctrl => (UblxAction::PreviewBottom, None),
        KeyCode::Char('r' | 'R') if ctrl => (UblxAction::ReloadConfig, None),
        KeyCode::Char('s' | 'S') if ctrl => (UblxAction::TakeSnapshot, None),
        KeyCode::Char('v' | 'V') if ctrl => (UblxAction::CycleRightPane, None),
        KeyCode::Char('l' | 'L') if ctrl => (UblxAction::LensMenu, None),
        KeyCode::Char('G') if shift => (UblxAction::ListBottom, None),
        KeyCode::Char('g') if !shift && !ctrl => {
            if ctx.last_key_for_double == Some('g') {
                (UblxAction::ListTop, None)
            } else {
                (UblxAction::Noop, Some('g'))
            }
        }
        KeyCode::Char(c) if shift => (UblxAction::SearchChar(c), None),
        KeyCode::Char(c) => {
            let a = match c {
                ' ' => UblxAction::SpaceMenu,
                '1' => UblxAction::MainModeSnapshot,
                '2' => UblxAction::MainModeDelta,
                '9' if ctx.tabs.duplicates => UblxAction::MainModeDuplicates,
                '3' if ctx.tabs.lenses => UblxAction::MainModeLenses,
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
#[must_use]
pub fn search_consumes(action: UblxAction) -> bool {
    matches!(
        action,
        UblxAction::SearchClear
            | UblxAction::SearchSubmit
            | UblxAction::SearchBackspace
            | UblxAction::SearchChar(_)
    )
}
