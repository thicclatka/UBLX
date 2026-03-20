//! TUI integration smoke: one frame through [`ublx::render::draw_ublx_frame`] on a
//! [`ratatui::backend::TestBackend`] (no crossterm, no real terminal).
//!
//! This exercises the same render pipeline as the live app (`themes::set_current`, tab bar, snapshot
//! panes) without running `main_app_loop` or subprocesses.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ublx::config::LayoutOverlay;
use ublx::layout::setup::{RightPaneContent, UblxState, ViewContents, ViewData};
use ublx::render::{DrawFrameArgs, draw_ublx_frame};

#[test]
fn draw_one_snapshot_frame_renders_main_chrome() {
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend).expect("test terminal");

    let mut state = UblxState::new();
    let view = ViewData {
        filtered_categories: vec!["misc".to_string()],
        contents: ViewContents::SnapshotIndices(vec![]),
        category_list_len: 1,
        content_len: 0,
    };
    let right = RightPaneContent::empty();
    let layout = LayoutOverlay::default();
    let args = DrawFrameArgs {
        delta_data: None,
        all_rows: None,
        dir_to_ublx: None,
        theme_name: None,
        transparent: false,
        layout: &layout,
        latest_snapshot_ns: None,
        dev: false,
        duplicate_groups: None,
        lens_names: None,
    };

    terminal
        .draw(|f| {
            draw_ublx_frame(f, &mut state, &view, &right, &args);
        })
        .expect("draw one frame");

    let buf: &Buffer = terminal.backend().buffer();
    let flat: String = buf.content().iter().map(|c| c.symbol()).collect();

    assert!(
        flat.contains("UBLX") || flat.contains("Snapshot"),
        "expected tab row / brand in buffer; sample: {}",
        flat.chars().take(400).collect::<String>()
    );
}
