//! Block → [`ratatui::text::Text`]: flow-wrap, tables, code fences, rules.

use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use rayon::prelude::*;

use crate::config::PARALLEL;
use crate::layout::themes::{self, Theme};

use super::flow_wrap;
use super::md_tables::render_markdown_table_lines;
use super::types::{Block, MarkdownDoc, StyledLines};

const FENCED_CODE_BG_PCT: f32 = 0.20;

fn fenced_code_line_style(theme: &Theme) -> Style {
    let bg = themes::lighten_rgb(theme.background, FENCED_CODE_BG_PCT);
    Style::default().fg(theme.text).bg(bg)
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

pub(crate) fn block_to_lines(
    block: &Block,
    width: u16,
    next_block: Option<&Block>,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    match block {
        Block::Heading { lines: content, .. } | Block::Paragraph(content) => {
            lines.push_lines(&flow_wrap::wrap_flow_block(content.clone(), width));
            lines.push_blank();
        }
        Block::Code { lang: _, text } => {
            let st = fenced_code_line_style(theme);
            let w = width as usize;
            // Top of box: blank line with same background (where opening ``` was)
            lines.push(Line::from(Span::styled(" ".repeat(w), st)));
            for line in text.lines() {
                // Pad line to full width so background spans the row (box effect)
                let padded = format!("{line:<w$}");
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
            lines.push_lines(&flow_wrap::wrap_list_item_lines(content.clone(), width));
            if !next_block.is_some_and(|b| matches!(b, Block::ListItem { .. })) {
                lines.push_blank();
            }
        }
        Block::Table { header, rows } => {
            lines.extend(render_markdown_table_lines(header, rows, width));
            lines.push_blank();
        }
        Block::Quote(s) => {
            lines.push_lines(&flow_wrap::wrap_quote_block(s, width));
            lines.push_blank();
        }
        Block::Rule => {
            let w = width as usize;
            lines.push(Line::from(RULE_CHAR.to_string().repeat(w)));
            lines.push_blank();
        }
        Block::Html(s) => {
            lines.push_lines(&flow_wrap::wrap_flow_block(
                vec![Line::from(s.clone())],
                width,
            ));
        }
    }
    lines
}

impl MarkdownDoc {
    #[must_use]
    pub fn to_text(&self, width: u16) -> Text<'static> {
        let theme = themes::current();

        let mut lines: Vec<Line<'static>> = if self.blocks.len() >= PARALLEL.markdown_blocks {
            self.blocks
                .par_iter()
                .enumerate()
                .flat_map(|(i, block)| {
                    let next = self.blocks.get(i + 1);
                    block_to_lines(block, width, next, theme)
                })
                .collect()
        } else {
            self.blocks
                .iter()
                .enumerate()
                .flat_map(|(i, block)| {
                    let next = self.blocks.get(i + 1);
                    block_to_lines(block, width, next, theme)
                })
                .collect()
        };
        trim_trailing_blank_line(&mut lines);
        Text::from(lines)
    }
}

fn trim_trailing_blank_line(lines: &mut Vec<Line<'static>>) {
    if lines.last().is_some_and(|l| l.width() == 0) {
        lines.pop();
    }
}
