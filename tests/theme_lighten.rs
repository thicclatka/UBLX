//! Theme HSL lightening — optional values dump for palette maintenance.

use ratatui::style::Color;
use ublx::layout::themes::lighten_rgb;

/// Run with: `cargo test print_lighten_values -- --nocapture`
/// to regenerate RGB values for `popup_bg` (5%) and `node_bg` (10%) from each theme background for palettes.rs.
#[test]
fn print_lighten_values() {
    let backgrounds: &[(&str, u8, u8, u8)] = &[
        ("Shadow Index", 0, 0, 0),
        ("Oblivion Ink", 10, 25, 47),
        ("Garden Unseen", 0, 42, 21),
        ("Burning Glyph", 42, 0, 0),
        ("Golden Delirium", 42, 42, 0),
        ("Tangerine Memory", 42, 26, 0),
        ("Purple Haze", 13, 0, 26),
        ("Silent Page", 255, 255, 255),
    ];
    for (name, r, g, b) in backgrounds {
        let bg = Color::Rgb(*r, *g, *b);
        let popup = lighten_rgb(bg, 0.05);
        let node = lighten_rgb(bg, 0.10);
        let Color::Rgb(pr, pg, pb) = popup else {
            unreachable!("lighten_rgb preserves Rgb for Rgb input")
        };
        let Color::Rgb(nr, ng, nb) = node else {
            unreachable!("lighten_rgb preserves Rgb for Rgb input")
        };
        println!(
            "{}: popup/notif Color::Rgb({}, {}, {}), node_bg Color::Rgb({}, {}, {})",
            name, pr, pg, pb, nr, ng, nb
        );
    }
}
