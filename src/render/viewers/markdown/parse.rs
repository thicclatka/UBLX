//! Pulldown-cmark event loop: markdown string → [`MarkdownDoc`](super::types::MarkdownDoc).

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::layout::themes;

use super::rich_utils::{RichBuilder, RichKind, handle_break, link_trailing_glyph_for_dest};
use super::types::{Block, MarkdownDoc, StyledLines};

const NL_CHAR: char = '\n';

fn replace_newlines(s: &str) -> String {
    s.replace(NL_CHAR, " ")
}

/// Bullets for unordered lists by nesting depth: -, •, +, then repeat.
const LIST_BULLETS: [char; 3] = ['-', '•', '+'];

fn prepend_list_bullet(lines: &mut StyledLines, list_ordered: bool, list_depth: usize) {
    let bullet = if list_ordered {
        '•'
    } else {
        LIST_BULLETS[list_depth.saturating_sub(1) % LIST_BULLETS.len()]
    };
    let indent = "  ".repeat(list_depth.saturating_sub(1));
    let prefix = format!("{indent}{bullet} ");
    let fg = Style::default().fg(themes::current().text);
    if let Some(first) = lines.first_mut() {
        let mut new_spans = vec![Span::styled(prefix.clone(), fg)];
        new_spans.extend(first.iter().cloned());
        *first = Line::from(new_spans);
    } else {
        lines.push(Line::from(vec![Span::styled(prefix, fg)]));
    }
}

/// Builds a [`Block`] when a block-level pulldown tag closes (`TagEnd::Paragraph`, `Heading`, etc.).
fn block_from_closed_tag(
    start_tag: &Tag<'_>,
    code_lang: Option<String>,
    text: String,
    rich: &mut Option<RichBuilder>,
    list_ordered: bool,
    list_depth: usize,
) -> Option<Block> {
    match start_tag {
        Tag::Heading { level, .. } => {
            let lines = rich.take().map_or_else(
                || vec![Line::from(replace_newlines(&text))],
                RichBuilder::finish,
            );
            Some(Block::Heading {
                level: *level as u8,
                lines,
            })
        }
        Tag::Paragraph => {
            let lines = rich
                .take()
                .map_or_else(|| vec![Line::from(text.clone())], RichBuilder::finish);
            Some(Block::Paragraph(lines))
        }
        Tag::CodeBlock(..) => {
            rich.take();
            Some(Block::Code {
                lang: code_lang,
                text,
            })
        }
        Tag::Item => {
            let mut lines = rich
                .take()
                .map_or_else(|| vec![Line::from(text.clone())], RichBuilder::finish);
            prepend_list_bullet(&mut lines, list_ordered, list_depth);
            Some(Block::ListItem {
                ordered: list_ordered,
                depth: list_depth.saturating_sub(1),
                prefix: String::new(),
                lines,
            })
        }
        Tag::BlockQuote(_) => {
            rich.take();
            Some(Block::Quote(text))
        }
        Tag::HtmlBlock => {
            rich.take();
            Some(Block::Html(text))
        }
        _ => None,
    }
}

/// Scratch state for [`parse_markdown`]: buffers, table assembly, rich-text stack.
struct MarkdownParseState<'a> {
    buf: String,
    block_tag: Option<(Tag<'a>, Option<String>)>,
    list_ordered: bool,
    list_depth: usize,
    table_header: Option<Vec<String>>,
    table_rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_table_cell: bool,
    table_link_depth: u32,
    table_link_pending_glyph: Option<char>,
    rich: Option<RichBuilder>,
}

impl<'a> MarkdownParseState<'a> {
    fn on_start(&mut self, tag: Tag<'a>) {
        match &tag {
            Tag::Strong
            | Tag::Emphasis
            | Tag::Strikethrough
            | Tag::Link { .. }
            | Tag::Image { .. } => {
                self.on_start_inline(&tag);
            }
            _ => self.on_start_block(tag),
        }
    }

    fn on_start_inline(&mut self, tag: &Tag<'a>) {
        match tag {
            Tag::Strong => {
                if let Some(r) = self.rich.as_mut() {
                    r.bold += 1;
                }
            }
            Tag::Emphasis => {
                if let Some(r) = self.rich.as_mut() {
                    r.italic += 1;
                }
            }
            Tag::Strikethrough => {
                if let Some(r) = self.rich.as_mut() {
                    r.strike += 1;
                }
            }
            Tag::Link { dest_url, .. } => {
                if let Some(r) = self.rich.as_mut() {
                    r.begin_link(dest_url.as_ref());
                } else if self.in_table_cell {
                    if self.table_link_depth == 0 {
                        self.table_link_pending_glyph =
                            Some(link_trailing_glyph_for_dest(dest_url.as_ref()));
                    }
                    self.table_link_depth += 1;
                }
            }
            Tag::Image { .. } => {
                if let Some(r) = self.rich.as_mut() {
                    r.begin_image();
                }
            }
            _ => {}
        }
    }

    fn on_start_block(&mut self, tag: Tag<'a>) {
        match &tag {
            Tag::CodeBlock(kind) => {
                self.buf.clear();
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                    CodeBlockKind::Indented => None,
                };
                self.block_tag = Some((tag, lang));
            }
            Tag::List(opt) => {
                self.list_ordered = opt.is_some();
                self.list_depth += 1;
                self.block_tag = Some((tag, None));
            }
            Tag::Table(_) => {
                self.table_header = None;
                self.table_rows.clear();
                self.current_row.clear();
                self.current_cell.clear();
                self.in_table_cell = false;
                self.table_link_depth = 0;
                self.table_link_pending_glyph = None;
            }
            Tag::TableCell => {
                self.in_table_cell = true;
                self.current_cell.clear();
                self.table_link_depth = 0;
                self.table_link_pending_glyph = None;
            }
            Tag::Paragraph => {
                self.buf.clear();
                self.rich = Some(RichBuilder::new(RichKind::Paragraph));
                self.block_tag = Some((tag, None));
            }
            Tag::Heading { level, .. } => {
                self.buf.clear();
                self.rich = Some(RichBuilder::new(RichKind::Heading {
                    level: *level as u8,
                }));
                self.block_tag = Some((tag, None));
            }
            Tag::Item => {
                self.buf.clear();
                self.rich = Some(RichBuilder::new(RichKind::Item));
                self.block_tag = Some((tag, None));
            }
            Tag::BlockQuote(_) | Tag::HtmlBlock => {
                self.buf.clear();
                self.rich = None;
                self.block_tag = Some((tag, None));
            }
            _ => {}
        }
    }

    /// Handle end tag. Returns `true` if the outer event loop should `continue` (skip block-close logic).
    fn on_end(&mut self, tag_end: TagEnd, blocks: &mut Vec<Block>) -> bool {
        match tag_end {
            TagEnd::Strong => {
                if let Some(r) = self.rich.as_mut() {
                    r.bold = r.bold.saturating_sub(1);
                }
            }
            TagEnd::Emphasis => {
                if let Some(r) = self.rich.as_mut() {
                    r.italic = r.italic.saturating_sub(1);
                }
            }
            TagEnd::Strikethrough => {
                if let Some(r) = self.rich.as_mut() {
                    r.strike = r.strike.saturating_sub(1);
                }
            }
            TagEnd::Link => {
                if let Some(r) = self.rich.as_mut() {
                    r.end_link();
                } else if self.in_table_cell && self.table_link_depth > 0 {
                    self.table_link_depth = self.table_link_depth.saturating_sub(1);
                    if self.table_link_depth == 0
                        && let Some(ch) = self.table_link_pending_glyph.take()
                    {
                        self.current_cell.push(' ');
                        self.current_cell.push(ch);
                    }
                }
            }
            TagEnd::Image => {
                if let Some(r) = self.rich.as_mut() {
                    r.end_image();
                }
            }
            TagEnd::TableCell => {
                self.current_row.push(self.current_cell.trim().to_string());
                self.current_cell.clear();
                self.in_table_cell = false;
            }
            TagEnd::TableRow => {
                self.table_rows.push(std::mem::take(&mut self.current_row));
            }
            TagEnd::TableHead => {
                // TableHead contains only TableCells (no TableRow), so header is in current_row here.
                self.table_header = Some(std::mem::take(&mut self.current_row));
            }
            TagEnd::Table => {
                let header = self.table_header.take().unwrap_or_default();
                let rows = std::mem::take(&mut self.table_rows);
                blocks.push(Block::Table { header, rows });
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
            }
            _ => {}
        }

        if matches!(
            tag_end,
            TagEnd::TableCell | TagEnd::TableRow | TagEnd::TableHead | TagEnd::Table
        ) {
            return true;
        }

        let block_level_end = matches!(
            tag_end,
            TagEnd::Paragraph
                | TagEnd::Heading(_)
                | TagEnd::CodeBlock
                | TagEnd::BlockQuote(_)
                | TagEnd::HtmlBlock
                | TagEnd::Item
        );
        if block_level_end {
            self.finish_block(blocks);
        }
        false
    }

    fn finish_block(&mut self, blocks: &mut Vec<Block>) {
        let text = self.buf.trim().to_string();
        self.buf.clear();
        let Some((start_tag, code_lang)) = self.block_tag.take() else {
            return;
        };
        if let Some(block) = block_from_closed_tag(
            &start_tag,
            code_lang,
            text,
            &mut self.rich,
            self.list_ordered,
            self.list_depth,
        ) {
            blocks.push(block);
        }
    }

    fn on_text(&mut self, t: &str) {
        if self.in_table_cell {
            self.current_cell.push_str(t);
        } else if let Some(r) = self.rich.as_mut() {
            r.push_text(t);
        } else {
            self.buf.push_str(t);
        }
    }

    fn on_code(&mut self, t: &str) {
        if self.in_table_cell {
            self.current_cell.push_str(t);
        } else if let Some(r) = self.rich.as_mut() {
            r.push_inline_code(t);
        } else {
            self.buf.push_str(t);
        }
    }

    fn on_html(&mut self, s: &str) {
        if self.in_table_cell {
            self.current_cell.push_str(s);
        } else if let Some(r) = self.rich.as_mut() {
            let lower = s.trim().to_ascii_lowercase();
            if lower == "</u>" {
                r.underline = r.underline.saturating_sub(1);
            } else if lower == "<u>" || lower.starts_with("<u ") {
                r.underline += 1;
            }
            // Do not push raw HTML into rich text (keeps `<u>` / links out of the viewer).
        } else {
            self.buf.push_str(s);
        }
    }

    fn on_break(&mut self) {
        handle_break(
            self.in_table_cell,
            &mut self.current_cell,
            &mut self.buf,
            &mut self.rich,
        );
    }
}

/// Parse markdown into a [`MarkdownDoc`]. GFM extensions (tables, strikethrough, etc.) are enabled
/// via [`Options`]; tables become [`Block::Table`], list items track nesting for bullet style.
#[must_use]
pub fn parse_markdown(s: &str) -> MarkdownDoc {
    let parser = Parser::new_ext(s, Options::all());
    let mut blocks = Vec::new();
    let mut st = MarkdownParseState {
        buf: String::new(),
        block_tag: None,
        list_ordered: false,
        list_depth: 0,
        table_header: None,
        table_rows: Vec::new(),
        current_row: Vec::new(),
        current_cell: String::new(),
        in_table_cell: false,
        table_link_depth: 0,
        table_link_pending_glyph: None,
        rich: None,
    };

    for event in parser {
        match event {
            Event::Start(tag) => st.on_start(tag),
            Event::End(tag_end) => {
                st.on_end(tag_end, &mut blocks);
            }
            Event::Text(t) => st.on_text(t.as_ref()),
            Event::Code(t) => st.on_code(t.as_ref()),
            Event::Html(t) | Event::InlineHtml(t) => st.on_html(t.as_ref()),
            Event::SoftBreak | Event::HardBreak => st.on_break(),
            Event::Rule => blocks.push(Block::Rule),
            _ => {}
        }
    }

    MarkdownDoc { blocks }
}
