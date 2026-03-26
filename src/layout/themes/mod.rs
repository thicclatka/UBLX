//! Theme and colors for the TUI. Multiple themes can be added; the active theme is chosen via opts (e.g. config `theme` key).

mod color_utils;
mod palettes;

pub use color_utils::{adjust_surface_rgb, darken_rgb, lighten_rgb};

use ratatui::style::Color;
use std::cell::RefCell;

pub use palettes::{DEFAULT_COLORS, OBLIVION_INK, theme_ordered_list, theme_selector_entries};

/// One row in the theme selector list: a non-selectable section label or a selectable theme.
#[derive(Clone, Copy)]
pub enum SelectorEntry {
    Section(&'static str),
    Item(&'static Theme),
}

/// Whether the theme is predominantly dark or light. Drives [`adjust_surface_rgb`]: dark themes *lighten*
/// auxiliary surfaces (code blocks, stripes); light themes *darken* them so the same `pct` reads as a
/// similar “step” off the page background.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Appearance {
    Dark,
    Light,
}

/// Default theme (Oblivion Ink). Used when opts theme is unset or "default".
pub const DEFAULT_THEME: &Theme = &OBLIVION_INK;

/// Named set of colors for the TUI. Extend with more fields as styles are themed.
#[derive(Clone, Debug)]
pub struct Theme {
    // #[allow(dead_code)]
    pub name: &'static str,
    /// See [`Appearance`] and [`adjust_surface_rgb`].
    pub appearance: Appearance,
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
    /// Node/footer powerline fill on **dark** themes. On **light** themes, footers use [`node_pill_background`] instead so pills don’t match the page.
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
    /// Color for theme selector swatch
    pub swatch: Color,
}

/// HSL step off [`Theme::background`] for light-theme footer/status powerline pills.
const LIGHT_THEME_NODE_PILL_PCT: f32 = 0.11;

/// Fill color for powerline footer nodes and related chrome. Dark themes: [`Theme::node_bg`]. Light themes: [`adjust_surface_rgb`] from background so bars read against the page.
#[must_use]
pub fn node_pill_background(theme: &Theme) -> Color {
    match theme.appearance {
        Appearance::Light => adjust_surface_rgb(
            theme.background,
            LIGHT_THEME_NODE_PILL_PCT,
            theme.appearance,
        ),
        Appearance::Dark => theme.node_bg,
    }
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
#[must_use]
pub fn current() -> &'static Theme {
    let name = CURRENT_THEME_NAME.with(|cell| cell.borrow().clone());
    get(name.as_deref())
}

/// Resolve config theme to the theme name to use. When config is `None`, empty, or `"default"`, returns [`DEFAULT_THEME`]'s name. Otherwise returns the config value. Use before passing to [`set_current`] or [`get`].
#[must_use]
pub fn theme_name_from_config(config_theme: Option<&str>) -> &str {
    match config_theme {
        None => DEFAULT_THEME.name,
        Some(s) => {
            let t = s.trim();
            if t.is_empty() || t == "default" {
                DEFAULT_THEME.name
            } else {
                t
            }
        }
    }
}

/// Resolve theme by name. Uses [`theme_name_from_config`] so `None` / empty / `"default"` use [`DEFAULT_THEME`]; then matches [`Theme::name`] (same strings as in [`theme_ordered_list`] / TOML).
#[must_use]
pub fn get(name: Option<&str>) -> &'static Theme {
    let n = theme_name_from_config(name);
    if n == DEFAULT_THEME.name {
        return DEFAULT_THEME;
    }
    theme_ordered_list()
        .iter()
        .copied()
        .find(|t| t.name == n)
        .unwrap_or(DEFAULT_THEME)
}
