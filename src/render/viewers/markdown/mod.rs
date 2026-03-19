//! Parsed markdown as simple block types for the viewer.
//!
//! Use [parse_markdown] to get a [MarkdownDoc]; then call [MarkdownDoc::to_text] for ratatui
//! with styled headings/lists/tables.
//!
//! Paragraphs, headings, and list items capture inline **bold**, *italic*, ~~strikethrough~~,
//! `<u>underline</u>`, and `` `inline code` `` (20% lightened background). Fenced code blocks
//! use a 20% lightened background and no ``` delimiters in the viewer.
//!
//! **Inline:** `[links](url)` append a trailing glyph from [`UiGlyphs`](crate::ui::UiGlyphs)
//! (`markdown_link`, or `markdown_attachment` for `.pdf`/archive/office-like paths in the URL);
//! `![images](url)` use a leading `markdown_image` before alt. Link text is underlined.
//!
//! **Table cells:** plain text plus **trailing link glyph** after `[text](url)` (same as inline);
//! no bold/italic in cells; images in cells still get no image glyph (raw alt only).
//!
//! **Omitted** (pulldown can emit tags; we ignore today): footnotes, definition lists, superscript,
//! subscript, YAML/metadata blocks, task-list markers, math.

mod core;
mod md_tables;
mod rich_utils;

pub use core::{is_markdown_path, parse_markdown, Block, MarkdownDoc, StyledLines};
