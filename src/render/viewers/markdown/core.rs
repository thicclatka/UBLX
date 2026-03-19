use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};

use crate::layout::themes;

use super::md_tables::render_markdown_table_lines;
use super::rich_utils::{RichBuilder, RichKind, handle_break, link_trailing_glyph_for_dest};

pub fn is_markdown_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

const NL_CHAR: char = '\n';

fn replace_newlines(s: &str) -> String {
    s.replace(NL_CHAR, " ")
}

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

/// Max characters per cell in markdown tables (longer than CSV).
/// Bullets for unordered lists by nesting depth: -, •, +, then repeat.
const LIST_BULLETS: [char; 3] = ['-', '•', '+'];

const FENCED_CODE_BG_PCT: f32 = 0.20;

fn prepend_list_bullet(lines: &mut StyledLines, list_ordered: bool, list_depth: usize) {
    let bullet = if list_ordered {
        '•'
    } else {
        LIST_BULLETS[list_depth.saturating_sub(1) % LIST_BULLETS.len()]
    };
    let indent = "  ".repeat(list_depth.saturating_sub(1));
    let prefix = format!("{}{} ", indent, bullet);
    let fg = Style::default().fg(themes::current().text);
    if let Some(first) = lines.first_mut() {
        let mut new_spans = vec![Span::styled(prefix.clone(), fg)];
        new_spans.extend(first.iter().cloned());
        *first = Line::from(new_spans);
    } else {
        lines.push(Line::from(vec![Span::styled(prefix, fg)]));
    }
}

/// Builds a [Block] when a block-level pulldown tag closes (`TagEnd::Paragraph`, `Heading`, etc.).
fn block_from_closed_tag(
    start_tag: Tag<'_>,
    code_lang: Option<String>,
    text: String,
    rich: &mut Option<RichBuilder>,
    list_ordered: bool,
    list_depth: usize,
) -> Option<Block> {
    match start_tag {
        Tag::Heading { level, .. } => {
            let lines = rich
                .take()
                .map(RichBuilder::finish)
                .unwrap_or_else(|| vec![Line::from(replace_newlines(&text))]);
            Some(Block::Heading {
                level: level as u8,
                lines,
            })
        }
        Tag::Paragraph => {
            let lines = rich
                .take()
                .map(RichBuilder::finish)
                .unwrap_or_else(|| vec![Line::from(text.clone())]);
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
                .map(RichBuilder::finish)
                .unwrap_or_else(|| vec![Line::from(text.clone())]);
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

/// Parse markdown into a [MarkdownDoc]. GFM extensions (tables, strikethrough, etc.) are enabled
/// via [Options]; tables become [Block::Table], list items track nesting for bullet style.
pub fn parse_markdown(s: &str) -> MarkdownDoc {
    let mut blocks = Vec::new();
    let mut parser = Parser::new_ext(s, Options::all());
    let mut buf = String::new();
    let mut block_tag: Option<(Tag<'_>, Option<String>)> = None;
    let mut list_ordered = false;
    let mut list_depth: usize = 0;
    let mut table_header: Option<Vec<String>> = None;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut in_table_cell = false;
    let mut table_link_depth: u32 = 0;
    let mut table_link_pending_glyph: Option<char> = None;
    let mut rich: Option<RichBuilder> = None;

    for event in parser.by_ref() {
        match event {
            Event::Start(tag) => match &tag {
                Tag::CodeBlock(kind) => {
                    buf.clear();
                    let lang = match kind {
                        CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                        CodeBlockKind::Indented => None,
                    };
                    block_tag = Some((tag, lang));
                }
                Tag::List(opt) => {
                    list_ordered = opt.is_some();
                    list_depth += 1;
                    block_tag = Some((tag, None));
                }
                Tag::Table(_) => {
                    table_header = None;
                    table_rows.clear();
                    current_row.clear();
                    current_cell.clear();
                    in_table_cell = false;
                    table_link_depth = 0;
                    table_link_pending_glyph = None;
                }
                Tag::TableHead => {}
                Tag::TableCell => {
                    in_table_cell = true;
                    current_cell.clear();
                    table_link_depth = 0;
                    table_link_pending_glyph = None;
                }
                Tag::Paragraph => {
                    buf.clear();
                    rich = Some(RichBuilder::new(RichKind::Paragraph));
                    block_tag = Some((tag, None));
                }
                Tag::Heading { level, .. } => {
                    buf.clear();
                    rich = Some(RichBuilder::new(RichKind::Heading {
                        level: *level as u8,
                    }));
                    block_tag = Some((tag, None));
                }
                Tag::Item => {
                    buf.clear();
                    rich = Some(RichBuilder::new(RichKind::Item));
                    block_tag = Some((tag, None));
                }
                Tag::BlockQuote(_) | Tag::HtmlBlock => {
                    buf.clear();
                    rich = None;
                    block_tag = Some((tag, None));
                }
                Tag::Strong => {
                    if let Some(r) = rich.as_mut() {
                        r.bold += 1;
                    }
                }
                Tag::Emphasis => {
                    if let Some(r) = rich.as_mut() {
                        r.italic += 1;
                    }
                }
                Tag::Strikethrough => {
                    if let Some(r) = rich.as_mut() {
                        r.strike += 1;
                    }
                }
                Tag::Link { dest_url, .. } => {
                    if let Some(r) = rich.as_mut() {
                        r.begin_link(dest_url.as_ref());
                    } else if in_table_cell {
                        if table_link_depth == 0 {
                            table_link_pending_glyph =
                                Some(link_trailing_glyph_for_dest(dest_url.as_ref()));
                        }
                        table_link_depth += 1;
                    }
                }
                Tag::Image { .. } => {
                    if let Some(r) = rich.as_mut() {
                        r.begin_image();
                    }
                }
                _ => {}
            },
            Event::End(tag_end) => {
                match tag_end {
                    TagEnd::Strong => {
                        if let Some(r) = rich.as_mut() {
                            r.bold = r.bold.saturating_sub(1);
                        }
                    }
                    TagEnd::Emphasis => {
                        if let Some(r) = rich.as_mut() {
                            r.italic = r.italic.saturating_sub(1);
                        }
                    }
                    TagEnd::Strikethrough => {
                        if let Some(r) = rich.as_mut() {
                            r.strike = r.strike.saturating_sub(1);
                        }
                    }
                    TagEnd::Link => {
                        if let Some(r) = rich.as_mut() {
                            r.end_link();
                        } else if in_table_cell && table_link_depth > 0 {
                            table_link_depth = table_link_depth.saturating_sub(1);
                            if table_link_depth == 0
                                && let Some(ch) = table_link_pending_glyph.take()
                            {
                                current_cell.push(' ');
                                current_cell.push(ch);
                            }
                        }
                    }
                    TagEnd::Image => {
                        if let Some(r) = rich.as_mut() {
                            r.end_image();
                        }
                    }
                    TagEnd::TableCell => {
                        current_row.push(current_cell.trim().to_string());
                        current_cell.clear();
                        in_table_cell = false;
                    }
                    TagEnd::TableRow => {
                        table_rows.push(std::mem::take(&mut current_row));
                    }
                    TagEnd::TableHead => {
                        // TableHead contains only TableCells (no TableRow), so header is in current_row here.
                        table_header = Some(std::mem::take(&mut current_row));
                    }
                    TagEnd::Table => {
                        let header = table_header.take().unwrap_or_default();
                        let rows = std::mem::take(&mut table_rows);
                        blocks.push(Block::Table { header, rows });
                    }
                    TagEnd::List(_) => {
                        list_depth = list_depth.saturating_sub(1);
                    }
                    _ => {}
                }
                if matches!(
                    tag_end,
                    TagEnd::TableCell | TagEnd::TableRow | TagEnd::TableHead | TagEnd::Table
                ) {
                    continue;
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
                    let text = buf.trim().to_string();
                    buf.clear();
                    if let Some((start_tag, code_lang)) = block_tag.take() {
                        let Some(block) = block_from_closed_tag(
                            start_tag,
                            code_lang,
                            text,
                            &mut rich,
                            list_ordered,
                            list_depth,
                        ) else {
                            continue;
                        };
                        blocks.push(block);
                    }
                }
            }
            Event::Text(t) => {
                if in_table_cell {
                    current_cell.push_str(t.as_ref());
                } else if let Some(r) = rich.as_mut() {
                    r.push_text(t.as_ref());
                } else {
                    buf.push_str(t.as_ref());
                }
            }
            Event::Code(t) => {
                if in_table_cell {
                    current_cell.push_str(t.as_ref());
                } else if let Some(r) = rich.as_mut() {
                    r.push_inline_code(t.as_ref());
                } else {
                    buf.push_str(t.as_ref());
                }
            }
            Event::Html(t) | Event::InlineHtml(t) => {
                let s = t.as_ref();
                if in_table_cell {
                    current_cell.push_str(s);
                } else if let Some(r) = rich.as_mut() {
                    let lower = s.trim().to_ascii_lowercase();
                    if lower == "</u>" {
                        r.underline = r.underline.saturating_sub(1);
                    } else if lower == "<u>" || lower.starts_with("<u ") {
                        r.underline += 1;
                    }
                    // Do not push raw HTML into rich text (keeps `<u>` / links out of the viewer).
                } else {
                    buf.push_str(s);
                }
            }
            Event::SoftBreak => {
                handle_break(in_table_cell, &mut current_cell, &mut buf, &mut rich);
            }
            Event::HardBreak => {
                handle_break(in_table_cell, &mut current_cell, &mut buf, &mut rich);
            }
            Event::Rule => blocks.push(Block::Rule),
            _ => {}
        }
    }

    MarkdownDoc { blocks }
}

fn fenced_code_line_style() -> Style {
    let t = themes::current();
    let bg = themes::lighten_rgb(t.background, FENCED_CODE_BG_PCT);
    Style::default().fg(t.text).bg(bg)
}

/// Box-drawing horizontal line for full-width rules.
const RULE_CHAR: char = '─';

trait LinePushExt {
    fn push_lines(&mut self, content: &StyledLines);
    fn push_blank(&mut self);
}

impl LinePushExt for Vec<Line<'static>> {
    fn push_lines(&mut self, content: &StyledLines) {
        self.extend(content.iter().cloned());
    }

    fn push_blank(&mut self) {
        self.push(Line::from(String::new()));
    }
}

fn block_to_lines(block: &Block, width: u16, next_block: Option<&Block>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match block {
        Block::Heading { lines: content, .. } => {
            lines.push_lines(content);
            lines.push_blank();
        }
        Block::Paragraph(content) => {
            lines.push_lines(content);
            lines.push_blank();
        }
        Block::Code { lang: _, text } => {
            let st = fenced_code_line_style();
            let w = width as usize;
            // Top of box: blank line with same background (where opening ``` was)
            lines.push(Line::from(Span::styled(" ".repeat(w), st)));
            for line in text.lines() {
                // Pad line to full width so background spans the row (box effect)
                let padded = format!("{:<width$}", line, width = w);
                lines.push(Line::from(Span::styled(padded, st)));
            }
            // Bottom of box: blank line with same background (where closing ``` was)
            lines.push(Line::from(Span::styled(" ".repeat(w), st)));
            lines.push_blank();
        }
        Block::ListItem {
            ordered: _,
            depth: _,
            prefix: _,
            lines: content,
        } => {
            lines.push_lines(content);
            if !next_block.is_some_and(|b| matches!(b, Block::ListItem { .. })) {
                lines.push_blank();
            }
        }
        Block::Table { header, rows } => {
            lines.extend(render_markdown_table_lines(header, rows, width));
            lines.push_blank();
        }
        Block::Quote(s) => {
            for line in s.lines() {
                lines.push(Line::from(format!("> {}", line)));
            }
            lines.push_blank();
        }
        Block::Rule => {
            let w = width as usize;
            lines.push(Line::from(RULE_CHAR.to_string().repeat(w)));
            lines.push_blank();
        }
        Block::Html(s) => {
            lines.push(Line::from(s.clone()));
        }
    }
    lines
}

impl MarkdownDoc {
    pub fn to_text(&self, width: u16) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = self
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(i, block)| {
                let next = self.blocks.get(i + 1);
                block_to_lines(block, width, next)
            })
            .collect();
        if lines.last().is_some_and(|l| l.width() == 0) {
            lines.pop();
        }
        Text::from(lines)
    }
}
