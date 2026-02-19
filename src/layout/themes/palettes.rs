//! AlephMetrics-derived theme palettes.
//! IDs: ShadowIndex, OblivionInk, GardenUnseen, BurningGlyph, GoldenDelirium, TangerineMemory, PurpleHaze, SilentPage.

use ratatui::style::Color;

use super::{Theme, ThemeOption};

/// Default delta colors (added / modified / removed). Use e.g. `DEFAULT_COLORS.green`.
pub const DEFAULT_COLORS: DefaultColors = DefaultColors {
    green: Color::Rgb(72, 187, 120),
    yellow: Color::Rgb(253, 203, 110),
    red: Color::Rgb(239, 68, 68),
    cyan: Color::Rgb(100, 255, 218),
    magenta: Color::Rgb(164, 95, 250),
    gray: Color::Rgb(128, 128, 128),
};

pub struct DefaultColors {
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
    pub cyan: Color,
    pub magenta: Color,
    pub gray: Color,
}

// ---- Shadow Index (black) ----
pub static SHADOW_INDEX: Theme = Theme {
    name: "Shadow Index",
    background: Color::Rgb(0, 0, 0),
    text: Color::Rgb(255, 255, 255),
    focused_border: Color::Rgb(153, 153, 153),
    tab_active_fg: Color::Rgb(153, 153, 153),
    tab_active_bg: Color::Rgb(102, 102, 102),
    tab_inactive_bg: Color::Rgb(45, 45, 45),
    search_text: Color::Rgb(128, 128, 128),
    hint: Color::Rgb(128, 128, 153),
    popup_bg: Color::Rgb(13, 12, 12),
    node_bg: Color::Rgb(27, 24, 24),
    notification_bg: Color::Rgb(13, 12, 12),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(128, 128, 153),
};

// ---- Oblivion Ink (blue) ----
pub static OBLIVION_INK: Theme = Theme {
    name: "Oblivion Ink",
    background: Color::Rgb(10, 25, 47),
    text: Color::Rgb(230, 241, 255),
    focused_border: Color::Rgb(100, 255, 218),
    tab_active_fg: Color::Rgb(100, 255, 218),
    tab_active_bg: Color::Rgb(59, 127, 217),
    tab_inactive_bg: Color::Rgb(17, 34, 64),
    search_text: Color::Rgb(100, 255, 218),
    hint: Color::Rgb(164, 95, 250),
    popup_bg: Color::Rgb(14, 35, 66),
    node_bg: Color::Rgb(17, 45, 85),
    notification_bg: Color::Rgb(14, 35, 66),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(164, 95, 250),
};

// ---- Garden Unseen (green) ----
pub static GARDEN_UNSEEN: Theme = Theme {
    name: "Garden Unseen",
    background: Color::Rgb(0, 42, 21),
    text: Color::Rgb(255, 255, 255),
    focused_border: Color::Rgb(130, 246, 198),
    tab_active_fg: Color::Rgb(130, 246, 198),
    tab_active_bg: Color::Rgb(49, 226, 165),
    tab_inactive_bg: Color::Rgb(10, 95, 53),
    search_text: Color::Rgb(98, 237, 181),
    hint: Color::Rgb(73, 67, 44),
    popup_bg: Color::Rgb(0, 65, 33),
    node_bg: Color::Rgb(0, 89, 44),
    notification_bg: Color::Rgb(0, 65, 33),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(73, 67, 44),
};

// ---- Burning Glyph (red) ----
pub static BURNING_GLYPH: Theme = Theme {
    name: "Burning Glyph",
    background: Color::Rgb(42, 0, 0),
    text: Color::Rgb(255, 255, 255),
    focused_border: Color::Rgb(246, 130, 130),
    tab_active_fg: Color::Rgb(246, 130, 130),
    tab_active_bg: Color::Rgb(226, 49, 49),
    tab_inactive_bg: Color::Rgb(131, 15, 15),
    search_text: Color::Rgb(237, 98, 98),
    hint: Color::Rgb(237, 181, 98),
    popup_bg: Color::Rgb(65, 0, 0),
    node_bg: Color::Rgb(89, 0, 0),
    notification_bg: Color::Rgb(65, 0, 0),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(237, 181, 98),
};

// ---- Golden Delirium (yellow) ----
pub static GOLDEN_DELIRIUM: Theme = Theme {
    name: "Golden Delirium",
    background: Color::Rgb(42, 42, 0),
    text: Color::Rgb(255, 255, 255),
    focused_border: Color::Rgb(246, 246, 130),
    tab_active_fg: Color::Rgb(246, 246, 130),
    tab_active_bg: Color::Rgb(226, 226, 49),
    tab_inactive_bg: Color::Rgb(131, 131, 15),
    search_text: Color::Rgb(237, 237, 98),
    hint: Color::Rgb(167, 107, 78),
    popup_bg: Color::Rgb(65, 65, 0),
    node_bg: Color::Rgb(89, 89, 0),
    notification_bg: Color::Rgb(65, 65, 0),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(167, 107, 78),
};

// ---- Tangerine Memory (orange) ----
pub static TANGERINE_MEMORY: Theme = Theme {
    name: "Tangerine Memory",
    background: Color::Rgb(42, 26, 0),
    text: Color::Rgb(255, 255, 255),
    focused_border: Color::Rgb(246, 198, 130),
    tab_active_fg: Color::Rgb(246, 198, 130),
    tab_active_bg: Color::Rgb(226, 125, 49),
    tab_inactive_bg: Color::Rgb(131, 59, 15),
    search_text: Color::Rgb(237, 155, 98),
    hint: Color::Rgb(226, 169, 157),
    popup_bg: Color::Rgb(65, 40, 0),
    node_bg: Color::Rgb(89, 55, 0),
    notification_bg: Color::Rgb(65, 40, 0),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(226, 169, 157),
};

// ---- Purple Haze (purple) ----
pub static PURPLE_HAZE: Theme = Theme {
    name: "Purple Haze",
    background: Color::Rgb(13, 0, 26),
    text: Color::Rgb(255, 255, 255),
    focused_border: Color::Rgb(241, 130, 246),
    tab_active_fg: Color::Rgb(241, 130, 246),
    tab_active_bg: Color::Rgb(192, 49, 226),
    tab_inactive_bg: Color::Rgb(82, 15, 131),
    search_text: Color::Rgb(221, 98, 237),
    hint: Color::Rgb(116, 80, 240),
    popup_bg: Color::Rgb(25, 0, 50),
    node_bg: Color::Rgb(37, 0, 74),
    notification_bg: Color::Rgb(25, 0, 50),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(116, 80, 240),
};

// ---- Silent Page (white) ----
pub static SILENT_PAGE: Theme = Theme {
    name: "Silent Page",
    background: Color::Rgb(255, 255, 255),
    text: Color::Rgb(0, 0, 0),
    focused_border: Color::Rgb(77, 77, 77),
    tab_active_fg: Color::Rgb(77, 77, 77),
    tab_active_bg: Color::Rgb(128, 128, 128),
    tab_inactive_bg: Color::Rgb(230, 230, 230),
    search_text: Color::Rgb(102, 102, 102),
    hint: Color::Rgb(102, 92, 82),
    popup_bg: Color::Rgb(255, 255, 255),
    node_bg: Color::Rgb(255, 255, 255),
    notification_bg: Color::Rgb(255, 255, 255),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(102, 92, 82),
};

/// All AlephMetrics palettes: (config id, theme). Config id matches JSON key (e.g. "ShadowIndex"); display name is theme.name (e.g. "Shadow Index"). "Oblivion_Ink" is an alias for the default.
static UBLX_THEMES: [(&str, &Theme); 9] = [
    ("ShadowIndex", &SHADOW_INDEX),
    ("OblivionInk", &OBLIVION_INK),
    ("Oblivion_Ink", &OBLIVION_INK),
    ("GardenUnseen", &GARDEN_UNSEEN),
    ("BurningGlyph", &BURNING_GLYPH),
    ("GoldenDelirium", &GOLDEN_DELIRIUM),
    ("TangerineMemory", &TANGERINE_MEMORY),
    ("PurpleHaze", &PURPLE_HAZE),
    ("SilentPage", &SILENT_PAGE),
];

pub fn all_ublx_themes() -> &'static [(&'static str, &'static Theme)] {
    &UBLX_THEMES
}

/// Unique theme options for the theme selector popup (8 themes; display_name = theme.name for toml).
macro_rules! theme_options_array {
    ($($theme:ident),* $(,)?) => {
        [
            $(ThemeOption {
                display_name: $theme.name,
                theme: &$theme,
            }),*
        ]
    };
}
static THEME_OPTIONS: [ThemeOption; 8] = theme_options_array!(
    SHADOW_INDEX,
    OBLIVION_INK,
    GARDEN_UNSEEN,
    BURNING_GLYPH,
    GOLDEN_DELIRIUM,
    TANGERINE_MEMORY,
    PURPLE_HAZE,
    SILENT_PAGE,
);

pub fn theme_options() -> &'static [ThemeOption] {
    &THEME_OPTIONS
}
