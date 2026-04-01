//! Integration tests for Settings local layout source ([`ublx::modules::settings::layout_overlay_for_local_editing`]).

use ublx::config::{LayoutOverlay, UblxOverlay};
use ublx::modules::settings::layout_overlay_for_local_editing;

#[test]
fn layout_overlay_no_local_uses_merged() {
    let merged = UblxOverlay {
        layout: Some(LayoutOverlay {
            left_pct: 10,
            middle_pct: 20,
            right_pct: 70,
        }),
        ..Default::default()
    };
    let got = layout_overlay_for_local_editing(None, &merged);
    assert_eq!(got.layout.as_ref().unwrap().left_pct, 10);
}

#[test]
fn layout_overlay_local_without_layout_section_uses_merged() {
    let local = UblxOverlay {
        layout: None,
        show_hidden_files: Some(true),
        ..Default::default()
    };
    let merged = UblxOverlay {
        layout: Some(LayoutOverlay {
            left_pct: 5,
            middle_pct: 15,
            right_pct: 80,
        }),
        ..Default::default()
    };
    let got = layout_overlay_for_local_editing(Some(&local), &merged);
    assert_eq!(got.layout.as_ref().unwrap().left_pct, 5);
}

#[test]
fn layout_overlay_local_with_layout_uses_local() {
    let local = UblxOverlay {
        layout: Some(LayoutOverlay {
            left_pct: 50,
            middle_pct: 25,
            right_pct: 25,
        }),
        ..Default::default()
    };
    let merged = UblxOverlay {
        layout: Some(LayoutOverlay {
            left_pct: 10,
            middle_pct: 20,
            right_pct: 70,
        }),
        ..Default::default()
    };
    let got = layout_overlay_for_local_editing(Some(&local), &merged);
    assert_eq!(got.layout.as_ref().unwrap().left_pct, 50);
}
