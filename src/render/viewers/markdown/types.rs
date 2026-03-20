//! Parsed markdown document and block types.

use ratatui::text::Line;

/// Lines with per-span styles (inline markdown).
pub type StyledLines = Vec<Line<'static>>;

/// One block of markdown (heading, paragraph, code block, list, etc.).
#[derive(Clone, Debug)]
pub enum Block {
    Heading {
        level: u8,
        lines: StyledLines,
    },
    Paragraph(StyledLines),
    Code {
        lang: Option<String>,
        text: String,
    },
    ListItem {
        ordered: bool,
        depth: usize,
        /// Prefix only (indent + bullet + space); body is in `lines`.
        prefix: String,
        lines: StyledLines,
    },
    /// GFM table: header row and body rows. Rendered with comfy-table like CSV.
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Quote(String),
    Rule,
    /// Raw HTML (you can skip or strip when printing).
    Html(String),
}

/// A document is a sequence of blocks.
#[derive(Clone, Debug, Default)]
pub struct MarkdownDoc {
    pub blocks: Vec<Block>,
}

crate::define_path_ext_predicate! {
    #[must_use]
    pub fn is_markdown_path(path: &str) -> bool {
        "md", "markdown"
    }
}
