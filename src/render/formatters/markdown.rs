//! Parsed markdown as simple block types so the viewer can decide how to render them.
//!
//! Use [parse_markdown] to get a [MarkdownDoc]; then call [MarkdownDoc::to_text] for ratatui
//! (with styled headings) or [MarkdownDoc::to_plain_string] for plain text.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

/// One block of markdown (heading, paragraph, code block, list, etc.).
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    Code {
        lang: Option<String>,
        text: String,
    },
    ListItem {
        ordered: bool,
        text: String,
    },
    Quote(String),
    Rule,
    /// Raw HTML (you can skip or strip when printing).
    Html(String),
}

/// A document is a sequence of blocks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkdownDoc {
    pub blocks: Vec<Block>,
}

/// Parse markdown into a [MarkdownDoc]. GFM extensions (tables, strikethrough, etc.) are enabled
/// via [Options]; table rows are currently emitted as plain paragraphs.
pub fn parse_markdown(s: &str) -> MarkdownDoc {
    let mut blocks = Vec::new();
    let mut parser = Parser::new_ext(s, Options::all());
    let mut buf = String::new();
    let mut block_tag: Option<(Tag<'_>, Option<String>)> = None; // (tag, optional code lang)
    let mut list_ordered: bool = false;

    for event in parser.by_ref() {
        match event {
            Event::Start(tag) => {
                buf.clear();
                match &tag {
                    Tag::CodeBlock(kind) => {
                        let lang = match kind {
                            CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                            CodeBlockKind::Indented => None,
                        };
                        block_tag = Some((tag, lang));
                    }
                    Tag::List(opt) => {
                        list_ordered = opt.is_some();
                        block_tag = Some((tag, None));
                    }
                    _ => block_tag = Some((tag, None)),
                }
            }
            Event::End(_tag_end) => {
                let text = buf.trim().to_string();
                buf.clear();
                if let Some((start_tag, code_lang)) = block_tag.take() {
                    let block = match start_tag {
                        Tag::Heading { level, .. } => Block::Heading {
                            level: level as u8,
                            text: text.replace('\n', " "),
                        },
                        Tag::Paragraph => Block::Paragraph(text),
                        Tag::CodeBlock(..) => Block::Code {
                            lang: code_lang,
                            text,
                        },
                        Tag::Item => Block::ListItem {
                            ordered: list_ordered,
                            text,
                        },
                        Tag::BlockQuote(_) => Block::Quote(text),
                        Tag::HtmlBlock => Block::Html(text),
                        _ => continue,
                    };
                    blocks.push(block);
                }
            }
            Event::Text(t) => buf.push_str(t.as_ref()),
            Event::Code(t) => buf.push_str(t.as_ref()),
            Event::Html(t) | Event::InlineHtml(t) => buf.push_str(t.as_ref()),
            Event::SoftBreak => buf.push('\n'),
            Event::HardBreak => buf.push('\n'),
            Event::Rule => blocks.push(Block::Rule),
            _ => {}
        }
    }

    MarkdownDoc { blocks }
}

/// Style for heading lines in the TUI (bold + cyan).
fn heading_style(level: u8) -> Style {
    let mut s = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Cyan);
    if level == 1 {
        s = s.add_modifier(Modifier::UNDERLINED);
    }
    s
}

/// Box-drawing horizontal line for full-width rules.
const RULE_CHAR: char = '─';

impl MarkdownDoc {
    /// Render as ratatui [Text] with headings styled (bold, cyan; H1 also underlined).
    /// `width` is the content area width, used to draw horizontal rules as a full line.
    pub fn to_text(&self, width: u16) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        for block in &self.blocks {
            match block {
                Block::Heading { level, text } => {
                    let prefix = "#".repeat(*level as usize);
                    let style = heading_style(*level);
                    lines.push(Line::from(Span::styled(
                        format!("{} {}", prefix, text),
                        style,
                    )));
                    lines.push(Line::from(String::new()));
                }
                Block::Paragraph(s) => {
                    lines.push(Line::from(s.clone()));
                    lines.push(Line::from(String::new()));
                }
                Block::Code { lang: _, text } => {
                    lines.push(Line::from("```".to_string()));
                    for line in text.lines() {
                        lines.push(Line::from(line.to_string()));
                    }
                    lines.push(Line::from("```".to_string()));
                    lines.push(Line::from(String::new()));
                }
                Block::ListItem { ordered: _, text } => {
                    lines.push(Line::from(format!("- {}", text)));
                }
                Block::Quote(s) => {
                    for line in s.lines() {
                        lines.push(Line::from(format!("> {}", line)));
                    }
                    lines.push(Line::from(String::new()));
                }
                Block::Rule => {
                    let w = width as usize;
                    lines.push(Line::from(RULE_CHAR.to_string().repeat(w)));
                    lines.push(Line::from(String::new()));
                }
                Block::Html(s) => {
                    lines.push(Line::from(s.clone()));
                }
            }
        }
        if lines.last().is_some_and(|l| l.width() == 0) {
            lines.pop();
        }
        Text::from(lines)
    }

    /// Render the document as plain text (one block per logical line / paragraph).
    #[allow(dead_code)]
    pub fn to_plain_string(&self) -> String {
        let mut out = String::new();
        for block in &self.blocks {
            match block {
                Block::Heading { level, text } => {
                    let prefix = "#".repeat(*level as usize);
                    out.push_str(&format!("{} {}\n\n", prefix, text));
                }
                Block::Paragraph(s) => {
                    out.push_str(s);
                    out.push_str("\n\n");
                }
                Block::Code { lang: _, text } => {
                    out.push_str("```\n");
                    out.push_str(text);
                    out.push_str("\n```\n\n");
                }
                Block::ListItem { ordered: _, text } => {
                    out.push_str("- ");
                    out.push_str(text);
                    out.push('\n');
                }
                Block::Quote(s) => {
                    for line in s.lines() {
                        out.push_str("> ");
                        out.push_str(line);
                        out.push('\n');
                    }
                    out.push('\n');
                }
                Block::Rule => out.push_str("---\n\n"),
                Block::Html(s) => {
                    out.push_str(s);
                    out.push('\n');
                }
            }
        }
        out.trim_end().to_string()
    }
}
