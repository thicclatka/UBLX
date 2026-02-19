//! Unicode glyphs used by the TUI (powerline segments, blocks, etc.). Single place to change or document font requirements.

/// Unicode symbols used in layout/render. Nerd Fonts or similar may be needed for powerline characters.
pub struct UiGlyphs {
    /// Powerline-style segment: round left (curve on right). Used for tab nodes and status nodes.
    pub round_left: char,
    /// Powerline-style segment: round right (curve on left). Used for tab nodes and status nodes.
    pub round_right: char,
    /// Full block (e.g. theme selector swatch). U+2588.
    pub swatch_block: char,
}

impl UiGlyphs {
    pub const fn new() -> Self {
        Self {
            round_left: '\u{e0b6}',
            round_right: '\u{e0b4}',
            swatch_block: '\u{2588}',
        }
    }
}

pub const UI_GLYPHS: UiGlyphs = UiGlyphs::new();
