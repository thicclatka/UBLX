//! Markdown parse + `to_text` integration: blocks, fenced code, GFM tables, headings.

use ublx::render::viewers::markdown::{Block, MarkdownDoc, parse_markdown};

fn assert_lines_within_width(doc: &MarkdownDoc, w: u16) {
    let text = doc.to_text(w);
    for line in &text.lines {
        assert!(
            line.width() <= w as usize,
            "line {:?} exceeds width {}",
            line,
            w
        );
    }
}

#[test]
fn fenced_code_block_parses_and_respects_width() {
    let doc = parse_markdown(
        r"```rust
fn main() {}
```",
    );
    let code = doc.blocks.iter().find_map(|b| match b {
        Block::Code { lang, text } => Some((lang.as_deref(), text.as_str())),
        _ => None,
    });
    assert!(code.is_some(), "expected Block::Code, got {:?}", doc.blocks);
    let (lang, text) = code.unwrap();
    assert_eq!(lang, Some("rust"));
    assert!(text.contains("main"));
    assert_lines_within_width(&doc, 40);
}

#[test]
fn gfm_table_parses_header_and_rows() {
    let md = r"| H1 | H2 |
| --- | --- |
| a | b |
";
    let doc = parse_markdown(md);
    let table = doc.blocks.iter().find_map(|b| match b {
        Block::Table { header, rows } => Some((header.as_slice(), rows.as_slice())),
        _ => None,
    });
    assert!(
        table.is_some(),
        "expected Block::Table, got {:?}",
        doc.blocks
    );
    let (header, rows) = table.unwrap();
    assert_eq!(header, &["H1", "H2"]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], vec!["a", "b"]);
}

#[test]
fn heading_block_has_level() {
    let doc = parse_markdown("# Title\n\nBody.");
    let heading = doc.blocks.iter().find_map(|b| match b {
        Block::Heading { level, .. } => Some(*level),
        _ => None,
    });
    assert_eq!(heading, Some(1));
}

#[test]
fn heading_to_text_respects_width() {
    let doc = parse_markdown("# Short\n");
    assert_lines_within_width(&doc, 20);
}
