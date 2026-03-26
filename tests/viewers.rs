//! Right-pane viewers: CSV hints, markdown, comfy tables, image labels, zahir JSON helpers.

use std::path::Path;

use ublx::integrations::{
    ZahirFileType as FileType, ZahirOutput, file_type_from_metadata_name,
    zahir_output_to_json_for_path,
};
use ublx::render::viewers::csv_handler::parse_csv;
use ublx::render::viewers::image::label_body_error;
use ublx::render::viewers::markdown::{Block, MarkdownDoc, parse_markdown};
use ublx::render::viewers::pretty_tables::{
    VIEWER_TABLE_NO_WRAP_COL_MAX_CHARS, prepare_multiline_grid, word_wrap_cell_text, word_wrap_text,
};
use ublx::ui::UI_GLYPHS;

// --- CSV --------------------------------------------------------------------

#[test]
fn path_hint_tsv_uses_tab() {
    let raw = "a\tb\nc\td\n";
    let rows = parse_csv(raw, Some("dir/file.tsv")).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["c", "d"]]);
}

#[test]
fn path_hint_psv_uses_pipe() {
    let raw = "a|b\nc|d\n";
    let rows = parse_csv(raw, Some("data.psv")).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["c", "d"]]);
}

#[test]
fn comma_without_hint_still_parses() {
    let raw = "a,b\n1,2\n";
    let rows = parse_csv(raw, None).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"], vec!["1", "2"]]);
}

// --- Markdown ---------------------------------------------------------------

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
    let doc =
        parse_markdown("> this is a long quoted line that must wrap when the viewport is narrow");
    let text = doc.to_text(12);
    for line in &text.lines {
        assert!(line.width() <= 12, "quote line {:?} exceeds width 12", line);
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

// --- Image viewer -----------------------------------------------------------

#[test]
fn label_body_error_includes_markdown_image_glyph() {
    let s = label_body_error("not found");
    assert!(s.contains("not found"));
    assert!(s.contains(UI_GLYPHS.markdown_image));
}

// --- Pretty tables (comfy-table grid + word wrap) --------------------------

/// Line count for cells joined with `\n`. `str::lines()` skips a final empty line after `\n`.
fn cell_visual_lines(s: &str) -> usize {
    s.split('\n').count()
}

#[test]
fn short_column_skips_wrap_but_pads() {
    let header = vec!["Long header that wraps".into(), "B".into()];
    let body = vec![vec![
        "first body line is long enough to wrap here yes".into(),
        "x".into(),
    ]];
    let (h, b) = prepare_multiline_grid(&header, &body, 24);
    assert!(h[0].contains('\n'), "first col should wrap: {:?}", h);
    let header_h = cell_visual_lines(&h[0]);
    assert_eq!(
        cell_visual_lines(&h[1]),
        header_h,
        "short header cell is not word-wrapped but padded to row height: {:?}",
        h
    );
    assert_eq!(h[1].split('\n').next(), Some("B"));
    assert!(
        h[1].split('\n').skip(1).all(|l| l.is_empty()),
        "padding lines after short header text should be blank: {:?}",
        h[1]
    );
    let row0 = &b[0];
    assert!(row0[0].contains('\n'));
    let n0 = cell_visual_lines(&row0[0]);
    assert_eq!(
        cell_visual_lines(&row0[1]),
        n0,
        "short column padded to same visual height: {:?}",
        row0[1]
    );
    assert_eq!(row0[1].split('\n').next(), Some("x"));
}

#[test]
fn header_with_spaces_breaks_at_words_not_mid_word() {
    let header = vec!["Completion Date".into(), "B".into()];
    let body = vec![vec![
        "x".repeat(VIEWER_TABLE_NO_WRAP_COL_MAX_CHARS + 1),
        "y".into(),
    ]];
    let (h, _) = prepare_multiline_grid(&header, &body, 24);
    let lines: Vec<&str> = h[0].split('\n').collect();
    assert!(
        lines.contains(&"Completion"),
        "expected whole word line, got {:?}",
        lines
    );
    assert!(
        lines.contains(&"Date"),
        "expected whole word line, got {:?}",
        lines
    );
    assert!(
        !lines.iter().any(|l| *l == "Comp" || *l == "leti"),
        "should not hard-break inside Completion: {:?}",
        lines
    );
}

#[test]
fn word_wrap_breaks_long_word() {
    let lines = word_wrap_text("abcdefghij", 4);
    assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
}

#[test]
fn word_wrap_spaces() {
    let lines = word_wrap_text("aa bb cc", 5);
    assert_eq!(lines, vec!["aa bb", "cc"]);
}

#[test]
fn word_wrap_cell_splits_isodate_on_hyphens_not_mid_digit() {
    let lines = word_wrap_cell_text("2025-10-25", 4);
    assert_eq!(lines, vec!["2025", "-10", "-25"]);
}

#[test]
fn word_wrap_cell_hyphen_long_word_fallback() {
    let lines = word_wrap_cell_text("abcdefghij", 4);
    assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
}

// --- Zahir wrapper ----------------------------------------------------------

#[test]
fn wrapper_matches_zahirscan_api() {
    assert_eq!(file_type_from_metadata_name("CSV"), Some(FileType::Csv));
    assert_eq!(
        file_type_from_metadata_name("Markdown"),
        Some(FileType::Markdown)
    );
}

#[test]
fn non_zahir_categories_miss() {
    assert_eq!(file_type_from_metadata_name("Directory"), None);
    assert_eq!(file_type_from_metadata_name("not a label"), None);
}

#[test]
fn zahir_json_for_path_injects_file_type_when_empty() {
    let o = ZahirOutput::default();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let full = root.join("src/lib.rs");
    let j = zahir_output_to_json_for_path(Some(&o), &full, "src/lib.rs");
    assert!(
        j.contains(r#""file_type":"Code""#),
        "expected path-based Code for .rs, got {j}"
    );
}
