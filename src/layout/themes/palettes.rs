//! AlephMetrics-derived theme palettes.
//! Config / UI theme name is [`Theme::name`] (e.g. `"Oblivion Ink"`).

use std::sync::LazyLock;

use ratatui::style::Color;

use super::{Appearance, SelectorEntry, Theme};

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
    appearance: Appearance::Dark,
    background: Color::Rgb(0, 0, 0),
    text: Color::Rgb(235, 236, 242),
    focused_border: Color::Rgb(153, 153, 153),
    tab_active_fg: Color::Rgb(235, 236, 242),
    tab_active_bg: Color::Rgb(48, 52, 68),
    tab_inactive_bg: Color::Rgb(4, 4, 6),
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

// ---- Archival Simulacra (dark: matrix terminal — black field, neon phosphor green) ----
pub static ARCHIVAL_SIMULACRA: Theme = Theme {
    name: "Archival Simulacra",
    appearance: Appearance::Dark,
    background: Color::Rgb(0, 0, 0),
    text: Color::Rgb(68, 255, 106),
    focused_border: Color::Rgb(140, 255, 120),
    tab_active_fg: Color::Rgb(0, 0, 0),
    tab_active_bg: Color::Rgb(72, 255, 112),
    tab_inactive_bg: Color::Rgb(0, 14, 4),
    search_text: Color::Rgb(48, 220, 88),
    hint: Color::Rgb(0, 140, 54),
    popup_bg: Color::Rgb(0, 10, 3),
    node_bg: Color::Rgb(0, 20, 7),
    notification_bg: Color::Rgb(0, 10, 3),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(110, 255, 130),
};

// ---- Oblivion Ink (blue) ----
pub static OBLIVION_INK: Theme = Theme {
    name: "Oblivion Ink",
    appearance: Appearance::Dark,
    background: Color::Rgb(10, 25, 47),
    text: Color::Rgb(193, 228, 219),
    focused_border: Color::Rgb(100, 255, 218),
    tab_active_fg: Color::Rgb(100, 255, 218),
    tab_active_bg: Color::Rgb(22, 58, 92),
    tab_inactive_bg: Color::Rgb(9, 22, 42),
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
    appearance: Appearance::Dark,
    background: Color::Rgb(0, 42, 21),
    text: Color::Rgb(230, 206, 182),
    focused_border: Color::Rgb(130, 246, 198),
    tab_active_fg: Color::Rgb(230, 206, 182),
    tab_active_bg: Color::Rgb(24, 92, 75),
    tab_inactive_bg: Color::Rgb(0, 40, 22),
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
    appearance: Appearance::Dark,
    background: Color::Rgb(42, 0, 0),
    text: Color::Rgb(210, 200, 238),
    focused_border: Color::Rgb(246, 130, 130),
    tab_active_fg: Color::Rgb(210, 200, 238),
    tab_active_bg: Color::Rgb(82, 44, 18),
    tab_inactive_bg: Color::Rgb(38, 0, 0),
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
    appearance: Appearance::Dark,
    background: Color::Rgb(42, 42, 0),
    text: Color::Rgb(252, 246, 188),
    focused_border: Color::Rgb(246, 246, 130),
    tab_active_fg: Color::Rgb(252, 246, 188),
    tab_active_bg: Color::Rgb(36, 64, 40),
    tab_inactive_bg: Color::Rgb(38, 38, 0),
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
    appearance: Appearance::Dark,
    background: Color::Rgb(42, 26, 0),
    text: Color::Rgb(255, 226, 205),
    focused_border: Color::Rgb(246, 198, 130),
    tab_active_fg: Color::Rgb(255, 226, 205),
    tab_active_bg: Color::Rgb(72, 38, 52),
    tab_inactive_bg: Color::Rgb(40, 24, 0),
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
    appearance: Appearance::Dark,
    background: Color::Rgb(13, 0, 26),
    text: Color::Rgb(234, 220, 255),
    focused_border: Color::Rgb(241, 130, 246),
    tab_active_fg: Color::Rgb(234, 220, 255),
    tab_active_bg: Color::Rgb(58, 18, 118),
    tab_inactive_bg: Color::Rgb(12, 0, 24),
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

// ---- Frozen Phrase (dark: Nordic polar night, frost blue chrome) ----
pub static FROZEN_PHRASE: Theme = Theme {
    name: "Frozen Phrase",
    appearance: Appearance::Dark,
    background: Color::Rgb(46, 52, 64),
    text: Color::Rgb(216, 232, 252),
    focused_border: Color::Rgb(129, 178, 208),
    tab_active_fg: Color::Rgb(36, 42, 54),
    tab_active_bg: Color::Rgb(206, 216, 233),
    tab_inactive_bg: Color::Rgb(38, 44, 56),
    search_text: Color::Rgb(143, 188, 188),
    hint: Color::Rgb(172, 186, 207),
    popup_bg: Color::Rgb(40, 48, 62),
    node_bg: Color::Rgb(54, 64, 82),
    notification_bg: Color::Rgb(40, 48, 62),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(180, 200, 224),
};

// ---- Babel Blend (dark: deep navy page; orange focus/search vs red tab chrome — hues separated by role) ----
pub static BABEL_BLEND: Theme = Theme {
    name: "Babel Blend",
    appearance: Appearance::Dark,
    background: Color::Rgb(12, 22, 46),
    text: Color::Rgb(236, 172, 158),
    focused_border: Color::Rgb(255, 145, 62),
    tab_active_fg: Color::Rgb(255, 252, 248),
    tab_active_bg: Color::Rgb(168, 42, 48),
    tab_inactive_bg: Color::Rgb(18, 30, 58),
    search_text: Color::Rgb(255, 178, 92),
    hint: Color::Rgb(232, 96, 88),
    popup_bg: Color::Rgb(16, 28, 54),
    node_bg: Color::Rgb(22, 36, 68),
    notification_bg: Color::Rgb(16, 28, 54),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(255, 118, 58),
};

// ---- Silent Sheet (white) ----
pub static SILENT_SHEET: Theme = Theme {
    name: "Silent Sheet",
    appearance: Appearance::Light,
    background: Color::Rgb(255, 255, 255),
    text: Color::Rgb(0, 0, 0),
    focused_border: Color::Rgb(77, 77, 77),
    tab_active_fg: Color::Rgb(255, 255, 255),
    tab_active_bg: Color::Rgb(58, 68, 92),
    tab_inactive_bg: Color::Rgb(242, 242, 244),
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

// ---- Parched Paper (light: dry manuscript paper, ink-green text) ----
pub static PARCHED_PAPER: Theme = Theme {
    name: "Parched Paper",
    appearance: Appearance::Light,
    background: Color::Rgb(253, 248, 240),
    text: Color::Rgb(24, 78, 52),
    focused_border: Color::Rgb(52, 130, 88),
    tab_active_fg: Color::Rgb(255, 255, 255),
    tab_active_bg: Color::Rgb(38, 92, 64),
    tab_inactive_bg: Color::Rgb(251, 246, 237),
    search_text: Color::Rgb(72, 115, 88),
    hint: Color::Rgb(130, 98, 62),
    popup_bg: Color::Rgb(253, 244, 231),
    node_bg: Color::Rgb(253, 245, 233),
    notification_bg: Color::Rgb(252, 243, 229),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(48, 105, 72),
};

// ---- Pale Mirror (light: frosted blue-lilac page, plum ink — no green/teal chrome) ----
pub static PALE_MIRROR: Theme = Theme {
    name: "Pale Mirror",
    appearance: Appearance::Light,
    background: Color::Rgb(242, 245, 253),
    text: Color::Rgb(42, 30, 56),
    focused_border: Color::Rgb(118, 78, 152),
    tab_active_fg: Color::Rgb(255, 255, 255),
    tab_active_bg: Color::Rgb(74, 48, 102),
    tab_inactive_bg: Color::Rgb(244, 246, 252),
    search_text: Color::Rgb(108, 92, 132),
    hint: Color::Rgb(138, 88, 118),
    popup_bg: Color::Rgb(235, 238, 248),
    node_bg: Color::Rgb(233, 236, 246),
    notification_bg: Color::Rgb(234, 237, 247),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(96, 68, 128),
};

// ---- Ochre Thread (light: pale sand page, rust ink) ----
pub static OCHRE_THREAD: Theme = Theme {
    name: "Ochre Thread",
    appearance: Appearance::Light,
    background: Color::Rgb(250, 242, 228),
    text: Color::Rgb(110, 50, 22),
    focused_border: Color::Rgb(178, 88, 34),
    tab_active_fg: Color::Rgb(255, 252, 248),
    tab_active_bg: Color::Rgb(92, 44, 24),
    tab_inactive_bg: Color::Rgb(246, 237, 220),
    search_text: Color::Rgb(148, 96, 68),
    hint: Color::Rgb(88, 102, 118),
    popup_bg: Color::Rgb(242, 232, 216),
    node_bg: Color::Rgb(240, 230, 214),
    notification_bg: Color::Rgb(241, 231, 215),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(162, 74, 30),
};

// ---- Cryptic Chai (light: tea-stained parchment, chocolate ink) ----
pub static CRYPTIC_CHAI: Theme = Theme {
    name: "Cryptic Chai",
    appearance: Appearance::Light,
    background: Color::Rgb(247, 238, 222),
    text: Color::Rgb(58, 38, 28),
    focused_border: Color::Rgb(152, 92, 48),
    tab_active_fg: Color::Rgb(255, 250, 244),
    tab_active_bg: Color::Rgb(78, 48, 36),
    tab_inactive_bg: Color::Rgb(244, 234, 218),
    search_text: Color::Rgb(124, 88, 68),
    hint: Color::Rgb(108, 98, 72),
    popup_bg: Color::Rgb(239, 228, 208),
    node_bg: Color::Rgb(237, 226, 205),
    notification_bg: Color::Rgb(238, 227, 206),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(138, 72, 42),
};

// ---- Asterion Code (light: cool blue-gray stone, blue-forward ink — not Parched’s warm cream + forest green) ----
pub static ASTERION_CODE: Theme = Theme {
    name: "Asterion Code",
    appearance: Appearance::Light,
    background: Color::Rgb(232, 240, 242),
    text: Color::Rgb(10, 72, 96),
    focused_border: Color::Rgb(26, 128, 168),
    tab_active_fg: Color::Rgb(248, 252, 255),
    tab_active_bg: Color::Rgb(16, 82, 108),
    tab_inactive_bg: Color::Rgb(226, 234, 237),
    search_text: Color::Rgb(42, 108, 138),
    hint: Color::Rgb(118, 92, 72),
    popup_bg: Color::Rgb(222, 232, 235),
    node_bg: Color::Rgb(220, 230, 233),
    notification_bg: Color::Rgb(221, 231, 234),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(18, 96, 128),
};

// ---- Rosy Infinity (light: very pale steel gray page, rose-gold ink and chrome) ----
pub static ROSY_INFINITY: Theme = Theme {
    name: "Rosy Infinity",
    appearance: Appearance::Light,
    background: Color::Rgb(232, 235, 240),
    text: Color::Rgb(112, 68, 76),
    focused_border: Color::Rgb(196, 128, 118),
    tab_active_fg: Color::Rgb(255, 250, 248),
    tab_active_bg: Color::Rgb(88, 46, 54),
    tab_inactive_bg: Color::Rgb(226, 229, 234),
    search_text: Color::Rgb(148, 98, 102),
    hint: Color::Rgb(118, 108, 122),
    popup_bg: Color::Rgb(226, 229, 235),
    node_bg: Color::Rgb(224, 227, 233),
    notification_bg: Color::Rgb(225, 228, 234),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(178, 108, 98),
};

// ---- Barley Bound (light: Gruvbox-light riff — buttercream pad, dark warm ink, teal focus, orange brand) ----
pub static BARLEY_BOUND: Theme = Theme {
    name: "Barley Bound",
    appearance: Appearance::Light,
    background: Color::Rgb(251, 241, 199),
    text: Color::Rgb(60, 56, 54),
    focused_border: Color::Rgb(69, 133, 136),
    tab_active_fg: Color::Rgb(251, 241, 199),
    tab_active_bg: Color::Rgb(80, 73, 69),
    tab_inactive_bg: Color::Rgb(235, 219, 178),
    search_text: Color::Rgb(121, 116, 14),
    hint: Color::Rgb(124, 111, 100),
    popup_bg: Color::Rgb(235, 219, 178),
    node_bg: Color::Rgb(232, 213, 172),
    notification_bg: Color::Rgb(234, 216, 175),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(214, 93, 14),
};

// ---- Cold Draft (light: Nord-light riff — snow page, polar-night ink, frost blue chrome) ----
pub static COLD_DRAFT: Theme = Theme {
    name: "Cold Draft",
    appearance: Appearance::Light,
    background: Color::Rgb(236, 239, 244),
    text: Color::Rgb(46, 52, 64),
    focused_border: Color::Rgb(94, 129, 172),
    tab_active_fg: Color::Rgb(236, 239, 244),
    tab_active_bg: Color::Rgb(59, 66, 82),
    tab_inactive_bg: Color::Rgb(229, 233, 240),
    search_text: Color::Rgb(129, 161, 193),
    hint: Color::Rgb(76, 86, 106),
    popup_bg: Color::Rgb(229, 233, 240),
    node_bg: Color::Rgb(226, 231, 238),
    notification_bg: Color::Rgb(228, 232, 239),
    delta_added: DEFAULT_COLORS.green,
    delta_mod: DEFAULT_COLORS.yellow,
    delta_removed: DEFAULT_COLORS.red,
    title_brand: Color::Rgb(136, 192, 208),
};

/// Every palette; order does not matter (derived lists sort by name within dark / light).
static ALL_THEMES: &[&Theme] = &[
    &ARCHIVAL_SIMULACRA,
    &BABEL_BLEND,
    &BURNING_GLYPH,
    &FROZEN_PHRASE,
    &GARDEN_UNSEEN,
    &GOLDEN_DELIRIUM,
    &OBLIVION_INK,
    &PURPLE_HAZE,
    &SHADOW_INDEX,
    &TANGERINE_MEMORY,
    &ASTERION_CODE,
    &BARLEY_BOUND,
    &COLD_DRAFT,
    &CRYPTIC_CHAI,
    &OCHRE_THREAD,
    &PALE_MIRROR,
    &PARCHED_PAPER,
    &ROSY_INFINITY,
    &SILENT_SHEET,
];

struct ThemeLists {
    /// Section rows + theme rows for the picker (dark A–Z, light A–Z).
    selector: Vec<SelectorEntry>,
    /// Same themes as `Item` rows only: dark A–Z, then light A–Z (for config / lookup / flat index).
    ordered: Vec<&'static Theme>,
}

static THEME_LISTS: LazyLock<ThemeLists> = LazyLock::new(|| {
    let mut dark: Vec<&'static Theme> = ALL_THEMES
        .iter()
        .copied()
        .filter(|t| t.appearance == Appearance::Dark)
        .collect();
    let mut light: Vec<&'static Theme> = ALL_THEMES
        .iter()
        .copied()
        .filter(|t| t.appearance == Appearance::Light)
        .collect();
    dark.sort_by_key(|t| t.name);
    light.sort_by_key(|t| t.name);

    let mut selector = Vec::new();
    selector.push(SelectorEntry::Section("Dark"));
    for &t in &dark {
        selector.push(SelectorEntry::Item(t));
    }
    selector.push(SelectorEntry::Section("Light"));
    for &t in &light {
        selector.push(SelectorEntry::Item(t));
    }

    let ordered: Vec<&'static Theme> = selector
        .iter()
        .filter_map(|e| match e {
            SelectorEntry::Item(t) => Some(*t),
            SelectorEntry::Section(_) => None,
        })
        .collect();

    ThemeLists { selector, ordered }
});

#[must_use]
pub fn theme_ordered_list() -> &'static [&'static Theme] {
    &THEME_LISTS.ordered
}

#[must_use]
pub fn theme_selector_entries() -> &'static [SelectorEntry] {
    &THEME_LISTS.selector
}
