//! Subprocess and full-frame smoke tests (binary + render pipeline without a real terminal).

use std::process::Command;

fn ublx_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ublx"))
}

#[test]
fn help_exits_zero() {
    let out = ublx_bin().arg("--help").output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ublx") || stdout.contains("DIR"),
        "stdout: {}",
        stdout
    );
}

#[test]
fn test_mode_in_empty_dir() {
    let tmp = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("ublx_integration_test_dir");
    let _ = std::fs::create_dir_all(&tmp);
    let out = ublx_bin().arg("--test").arg(&tmp).output().unwrap();
    assert!(
        out.status.success(),
        "exit: {:?}, stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let db = tmp.join(".ublx");
    assert!(db.exists(), "expected .ublx after --test run");
}

#[test]
fn draw_one_snapshot_frame_renders_main_chrome() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ublx::config::LayoutOverlay;
    use ublx::layout::setup::{RightPaneContent, UblxState, ViewContents, ViewData};
    use ublx::render::{DrawFrameArgs, draw_ublx_frame};

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
