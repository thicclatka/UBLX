//! Theme and colors for the TUI. Multiple themes can be added; the active theme is chosen via opts (e.g. config `theme` key).

mod color_utils;
mod palettes;

pub use color_utils::lighten_rgb;

use ratatui::style::Color;
use std::cell::RefCell;

pub use palettes::{DEFAULT_COLORS, OBLIVION_INK, all_ublx_themes, theme_options};

/// One theme in the selector: display name (for UI and for `theme = "..."` in toml) and reference.
#[derive(Clone, Copy)]
pub struct ThemeOption {
    pub display_name: &'static str,
    pub theme: &'static Theme,
}

/// Default theme (Oblivion Ink). Used when opts theme is unset or "default".
pub const DEFAULT_THEME: ThemeOption = ThemeOption {
    display_name: "Oblivion Ink",
    theme: &OBLIVION_INK,
};

/// Named set of colors for the TUI. Extend with more fields as styles are themed.
#[derive(Clone, Debug)]
pub struct Theme {
    // #[allow(dead_code)]
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
    /// Popup background (help box, theme selector, etc.)
    pub popup_bg: Color,
    /// Node/footer powerline color
    pub node_bg: Color,
    /// Toast/notification overlay background
    pub notification_bg: Color,
    /// Delta: added (green)
    pub delta_added: Color,
    /// Delta: modified (yellow)
    pub delta_mod: Color,
    /// Delta: removed (red)
    pub delta_removed: Color,
    /// Brand title (e.g. UBLX)
    pub title_brand: Color,
}

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

/// Resolve config theme to the theme name to use. When config is `None`, empty, or `"default"`, returns [DEFAULT_THEME_NAME]. Otherwise returns the config value. Use before passing to [set_current] or [get].
pub fn theme_name_from_config(config_theme: Option<&str>) -> &str {
    match config_theme {
        None => DEFAULT_THEME.display_name,
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || t == "default" {
                DEFAULT_THEME.display_name
            } else {
                t
            }
        }
    }
}

/// Resolve theme by name. Uses [theme_name_from_config] so `None` / empty / `"default"` use [default_theme]; then looks up by id or display name.
pub fn get(name: Option<&str>) -> &'static Theme {
    let n = theme_name_from_config(name);
    if n == DEFAULT_THEME.display_name {
        return DEFAULT_THEME.theme;
    }
    all_ublx_themes()
        .iter()
        .find(|(id, t)| *id == n || t.name == n)
        .map(|(_, t)| *t)
        .unwrap_or(DEFAULT_THEME.theme)
}
