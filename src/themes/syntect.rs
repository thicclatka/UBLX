//! Keys into syntect’s embedded [`syntect::highlighting::ThemeSet::load_defaults`] for the viewer.

/// String keys for the built-in themedump (`base16-ocean.dark`, `InspiredGitHub`, …).
#[derive(Clone, Copy, Debug)]
pub struct CodeThemeKeys {
    pub dark: &'static str,
    pub light: &'static str,
    /// If `dark` / `light` is missing from the set, use this (must exist in syntect defaults).
    pub fallback: &'static str,
}

/// Viewer highlighter: dark vs light from [`Appearance`], aligned with syntect defaults.
///
/// [`Appearance`]: crate::themes::Appearance
pub const SYNTECT_THEME_KEYS: CodeThemeKeys = CodeThemeKeys {
    dark: "base16-ocean.dark",
    light: "InspiredGitHub",
    fallback: "base16-ocean.dark",
};
