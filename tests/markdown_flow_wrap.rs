//! Markdown flow wrapping: paragraphs / lists / quotes sized to `to_text(width)` without ratatui re-wrap.

use ublx::render::viewers::markdown::parse_markdown;

#[test]
fn paragraph_wraps_to_width() {
    let doc = parse_markdown("aa bb cc dd ee");
    let text = doc.to_text(8);
    assert!(
        text.lines.len() >= 2,
        "expected wrapped lines, got {}",
        text.lines.len()
    );
    for line in &text.lines {
        assert!(line.width() <= 8, "line {:?} exceeds width 8", line);
    }
}

#[test]
fn blockquote_wrap_respects_width() {
    let doc = parse_markdown(
        "> this is a long quoted line that must wrap when the viewport is narrow",
    );
    let text = doc.to_text(12);
    for line in &text.lines {
        assert!(
            line.width() <= 12,
            "quote line {:?} exceeds width 12",
            line
        );
    }
}

#[test]
fn list_item_wrap_respects_width() {
    let doc = parse_markdown("- short prefix then many words here that should wrap across lines");
    let w = 18u16;
    let text = doc.to_text(w);
    for line in &text.lines {
        assert!(
            line.width() <= w as usize,
            "list line {:?} exceeds width {}",
            line,
            w
        );
    }
}
