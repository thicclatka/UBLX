//! Right-pane line count for JSON tabs matches [`ublx::render::kv_tables::content_height`].

use ublx::layout::setup::{RightPaneContent, RightPaneMode, UblxState};
use ublx::render::kv_tables::content_height;
use ublx::render::panes::viewer_total_lines;

#[test]
fn viewer_total_lines_kv_tables_matches_content_height() {
    let json = r#"{"field": "value"}"#;
    let mut state = UblxState::new();
    state.right_pane_mode = RightPaneMode::Metadata;
    let mut rc = RightPaneContent::empty();
    rc.metadata = Some(json.to_string());
    let w = 80u16;
    let n = viewer_total_lines(&rc, w, Some(json), &mut state);
    assert_eq!(n, content_height(json) as usize);
}
