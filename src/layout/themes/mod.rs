//! Theme and colors for the TUI. Multiple themes can be added; the active theme is chosen via opts (e.g. config `theme` key).

use ratatui::style::Color;
use std::cell::RefCell;

/// Theme name used when config does not set one.
pub const DEFAULT_THEME_NAME: &str = "default";

/// Named set of colors for the TUI. Extend with more fields as styles are themed.
#[derive(Clone, Debug)]
pub struct Theme {
    #[allow(dead_code)]
    pub name: &'static str,
    /// Background for the whole app (full frame).
    pub background: Color,
    /// Default foreground for body text (list items, paragraphs, search input).
    pub text: Color,
    /// Focused panel border (e.g. categories/contents).
    pub focused_border: Color,
    /// Active tab foreground.
    pub tab_active_fg: Color,
    /// Active tab background.
    pub tab_active_bg: Color,
    /// Inactive tab background.
    pub tab_inactive_bg: Color,
    /// "Search:" label text in status line.
    pub search_text: Color,
    /// Hint text (e.g. "Esc to clear").
    pub hint: Color,
    /// Node/footer powerline color.
    pub node_bg: Color,
    /// Delta: added (green).
    pub delta_added: Color,
    /// Delta: modified (yellow).
    pub delta_mod: Color,
    /// Delta: removed (red).
    pub delta_removed: Color,
    /// Brand title (e.g. UBLX).
    pub title_brand: Color,
    /// Background for node circle (powerline).
    pub node_circle_bg: Color,
}

/// Default theme (dark, cyan accent). Used when opts theme is unset or "default".
#[allow(dead_code)]
pub fn default_theme() -> &'static Theme {
    &DEFAULT_THEME
}

static DEFAULT_THEME: Theme = Theme {
    name: DEFAULT_THEME_NAME,
    background: Color::Black,
    text: Color::White,
    focused_border: Color::Cyan,
    tab_active_fg: Color::Cyan,
    tab_active_bg: Color::Rgb(70, 70, 90),
    tab_inactive_bg: Color::Rgb(45, 45, 45),
    search_text: Color::Yellow,
    hint: Color::Cyan,
    node_bg: Color::Rgb(55, 55, 65),
    delta_added: Color::Green,
    delta_mod: Color::Yellow,
    delta_removed: Color::Red,
    title_brand: Color::Magenta,
    node_circle_bg: Color::Black,
};

thread_local! {
    static CURRENT_THEME_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the theme name for the current frame (called at start of draw). Style functions use this.
pub fn set_current(name: Option<&str>) {
    CURRENT_THEME_NAME.with(|cell| {
        *cell.borrow_mut() = name.map(String::from);
    });
}

/// Current theme for this frame. Use in style functions.
pub fn current() -> &'static Theme {
    let name = CURRENT_THEME_NAME.with(|cell| cell.borrow().clone());
    get(name.as_deref())
}

/// Resolve theme by name. Returns the default theme for `None` or unknown names.
pub fn get(name: Option<&str>) -> &'static Theme {
    match name {
        Some(n) if n == DEFAULT_THEME_NAME => &DEFAULT_THEME,
        Some(_) => &DEFAULT_THEME,
        None => &DEFAULT_THEME,
    }
}
