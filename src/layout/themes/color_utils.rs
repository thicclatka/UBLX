//! Color helpers (HSL lightening, etc.).

use ratatui::style::Color;

use crate::utils::format::Epsilon;

const RGB_MAX: f32 = 255.0;
const HSL_SEGMENT_ANGLE: f32 = 60.0;

/// Lighten an RGB color in HSL space so it stays saturated (not dull). `pct` in 0..=1 moves lightness toward 1.0 and optionally boosts saturation. Non-RGB colors are returned unchanged.
pub fn lighten_rgb(color: Color, pct: f32) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    let p = pct.clamp(0.0, 1.0);
    let (h, s, l) = rgb_u8_to_hsl(r, g, b);
    // Move lightness toward 1.0; nudge saturation up so it doesn't wash out
    let new_l = l + (1.0 - l) * p;
    let new_s = (s + (1.0 - s) * p * 0.5).min(1.0);
    let (r2, g2, b2) = hsl_to_rgb_u8(h, new_s, new_l);
    Color::Rgb(r2, g2, b2)
}

/// RGB u8 [0,255] → HSL: H in [0, 360), S and L in [0, 1].
fn rgb_u8_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = f32::from(r) / RGB_MAX;
    let g = f32::from(g) / RGB_MAX;
    let b = f32::from(b) / RGB_MAX;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < Epsilon::COLOR {
        return (0.0, 0.0, l);
    }
    let s = if l <= 0.5 {
        (max - min) / (max + min)
    } else {
        (max - min) / (2.0 - max - min)
    };
    let h = if (max - r).abs() < Epsilon::COLOR {
        HSL_SEGMENT_ANGLE * ((g - b) / (max - min)).rem_euclid(6.0)
    } else if (max - g).abs() < Epsilon::COLOR {
        HSL_SEGMENT_ANGLE * ((b - r) / (max - min) + 2.0)
    } else {
        HSL_SEGMENT_ANGLE * ((r - g) / (max - min) + 4.0)
    };
    (h, s, l)
}

/// HSL (H in [0, 360), S and L in [0, 1]) → RGB u8.
fn hsl_to_rgb_u8(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    if s <= Epsilon::COLOR {
        let v = (l * RGB_MAX).round() as u8;
        return (v, v, v);
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / HSL_SEGMENT_ANGLE) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * RGB_MAX).round().clamp(0.0, RGB_MAX) as u8,
        ((g + m) * RGB_MAX).round().clamp(0.0, RGB_MAX) as u8,
        ((b + m) * RGB_MAX).round().clamp(0.0, RGB_MAX) as u8,
    )
}
