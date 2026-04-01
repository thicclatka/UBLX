//! Terminal background control via OSC **11** (set) and **111** (reset).
//!
//! When page opacity is below [`OPACITY_SOLID_MIN`], [`sync_osc11_page_background`] writes OSC 11 so
//! the **default background** matches the active theme RGB and the configured opacity. Terminals that
//! support it composite that color with the window’s own transparency, so the desktop wallpaper can
//! show through. Opacity at or above that threshold sends OSC 111 instead, restoring the terminal’s
//! usual dynamic background handling (aligned with solid fills in the Ratatui layer).
//!
//! Payload shape comes from [`crate::config::Osc11BackgroundFormat`]: `rgba(r,g,b,a)` (default) or
//! `#RRGGBBAA`. Some terminals (notably `WezTerm`) handle `rgba(...)` reliably; plain `#RRGGBBAA` may be
//! ignored depending on build—prefer `rgba` if blending misbehaves.

use std::io::{self, Write};

use crate::config::Osc11BackgroundFormat;
use crate::themes;

/// Minimum opacity treated as fully solid (`1.0` within float drift). Also used for TUI background fills and chrome when opacity is below this threshold.
pub const OPACITY_SOLID_MIN: f32 = 1.0 - 1e-4;

/// `true` when `v` counts as full opacity (omit from overlay, show as `1` in settings) — matches [`OPACITY_SOLID_MIN`].
#[inline]
#[must_use]
pub fn opacity_is_solid(v: f32) -> bool {
    v >= OPACITY_SOLID_MIN
}

/// Sync OSC 11 with page opacity: solid (`opacity` ≥ ~1) resets dynamic background; otherwise sets
/// `rgba(r,g,b,a)` from the active theme’s [`themes::Palette::background`].
///
/// `format`: config `opacity_format` (`rgba` default, `hex8` for `#RRGGBBAA`).
///
/// # Errors
///
/// Returns [`std::io::Error`] if writing or flushing **stdout** fails (broken pipe, etc.).
pub fn sync_osc11_page_background(
    theme_name: Option<&str>,
    opacity: f32,
    format: Osc11BackgroundFormat,
) -> io::Result<()> {
    let mut out = io::stdout();
    if opacity >= OPACITY_SOLID_MIN {
        return reset_osc_dynamic_background(&mut out);
    }
    let name = themes::theme_name_from_config(theme_name);
    let palette = themes::get(Some(name));
    let payload: Option<String> = match format {
        Osc11BackgroundFormat::Rgba => {
            themes::color_to_osc11_rgba_payload(palette.background, opacity)
        }
        Osc11BackgroundFormat::Hex8 => themes::color_to_osc11_hex8(palette.background, opacity),
    };
    let Some(payload) = payload else {
        return reset_osc_dynamic_background(&mut out);
    };
    write_osc11_background_payload(&mut out, &payload)
}

/// Set dynamic background via OSC 11 (`;` + payload, e.g. `rgba(10,25,47,0.1)`).
///
/// # Errors
///
/// Returns [`std::io::Error`] if the writer returns an error or [`Write::flush`] fails.
pub fn write_osc11_background_payload(w: &mut impl Write, payload: &str) -> io::Result<()> {
    write!(w, "\x1b]11;{payload}\x07")?;
    w.flush()
}

/// Reset dynamic text background (OSC 111). Pair with [`write_osc11_background_payload`].
///
/// # Errors
///
/// Returns [`std::io::Error`] if the writer returns an error or [`Write::flush`] fails.
pub fn reset_osc_dynamic_background(w: &mut impl Write) -> io::Result<()> {
    write!(w, "\x1b]111\x07")?;
    w.flush()
}
