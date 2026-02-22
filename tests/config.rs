//! Tests for config parsing (LayoutOverlay, UblxOverlay).

use ublx::config::{LayoutOverlay, UblxOverlay};

#[test]
fn layout_overlay_default() {
    let layout = LayoutOverlay::default();
    assert_eq!(layout.left_pct, 20);
    assert_eq!(layout.middle_pct, 30);
    assert_eq!(layout.right_pct, 50);
}

#[test]
fn layout_overlay_parse_toml() {
    let toml = r#"
[layout]
left_pct = 25
middle_pct = 35
right_pct = 40
"#;
    let overlay: UblxOverlay = toml::from_str(toml).unwrap();
    let layout = overlay.layout.unwrap();
    assert_eq!(layout.left_pct, 25);
    assert_eq!(layout.middle_pct, 35);
    assert_eq!(layout.right_pct, 40);
}

#[test]
fn ublx_overlay_merge_layout() {
    let mut base = UblxOverlay::default();
    let other = UblxOverlay {
        layout: Some(LayoutOverlay {
            left_pct: 10,
            middle_pct: 45,
            right_pct: 45,
        }),
        ..Default::default()
    };
    base.merge_from(&other);
    let layout = base.layout.unwrap();
    assert_eq!(layout.left_pct, 10);
    assert_eq!(layout.middle_pct, 45);
    assert_eq!(layout.right_pct, 45);
}
