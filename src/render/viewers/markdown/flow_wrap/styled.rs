//! Word-wrap [`ratatui::text::Line`]s while preserving per-character styles.

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::render::viewers::markdown::types::StyledLines;

/// One Unicode scalar with its [`Style`] (inline markdown glyph run).
pub type StyledGlyph = (char, Style);

#[must_use]
pub fn wrap_flow_block(lines: StyledLines, max_width: u16) -> StyledLines {
    let mw = max_width as usize;
    if mw == 0 {
        return lines;
    }
    let mut out: StyledLines = Vec::new();
    for line in lines {
        if line.width() <= mw {
            out.push(line);
            continue;
        }
        out.extend(wrap_single_line(&line, mw));
    }
    out
}

fn wrap_single_line(line: &Line<'static>, max_width: usize) -> StyledLines {
    let chars = line_to_styled_chars(line);
    if chars.is_empty() {
        return vec![Line::from(String::new())];
    }
    let words = split_words(chars);
    let mut result: StyledLines = Vec::new();
    let mut current: Vec<StyledGlyph> = Vec::new();
    let mut cur_w = 0usize;

    let flush_line = |buf: &mut Vec<StyledGlyph>, out: &mut StyledLines| {
        if !buf.is_empty() {
            out.push(chars_to_line(buf));
            buf.clear();
        }
    };

    for word in words {
        let ww = word_width(&word);
        // Gap before the next word when the line already has content (matches `plain::wrap_plain_words`).
        let gap = usize::from(!current.is_empty());
        if !current.is_empty() && cur_w + gap + ww > max_width {
            flush_line(&mut current, &mut result);
            cur_w = 0;
        }
        if ww > max_width {
            flush_line(&mut current, &mut result);
            cur_w = 0;
            for chunk in hard_break_word(&word, max_width) {
                if !chunk.is_empty() {
                    result.push(chars_to_line(&chunk));
                }
            }
            continue;
        }
        if !current.is_empty() {
            let space_style = current.last().map(|(_, s)| *s).unwrap_or_default();
            current.push((' ', space_style));
            cur_w += 1;
        }
        current.extend(word);
        cur_w += ww;
    }
    flush_line(&mut current, &mut result);
    if result.is_empty() {
        result.push(Line::from(String::new()));
    }
    result
}

fn line_to_styled_chars(line: &Line<'static>) -> Vec<StyledGlyph> {
    line.spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |ch| (ch, span.style)))
        .collect()
}

fn split_words(chars: Vec<StyledGlyph>) -> Vec<Vec<StyledGlyph>> {
    let mut words: Vec<Vec<StyledGlyph>> = Vec::new();
    let mut cur: Vec<StyledGlyph> = Vec::new();
    for (ch, st) in chars {
        if ch.is_whitespace() {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            continue;
        }
        cur.push((ch, st));
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

fn word_width(word: &[StyledGlyph]) -> usize {
    word.iter().map(|(c, _)| c.width().unwrap_or(0)).sum()
}

fn hard_break_word(word: &[StyledGlyph], max_width: usize) -> Vec<Vec<StyledGlyph>> {
    if word.is_empty() || max_width == 0 {
        return vec![word.to_vec()];
    }
    let mut chunks: Vec<Vec<StyledGlyph>> = Vec::new();
    let mut cur: Vec<StyledGlyph> = Vec::new();
    let mut w = 0usize;
    for pair in word {
        let cw = pair.0.width().unwrap_or(0);
        if w + cw > max_width && !cur.is_empty() {
            chunks.push(std::mem::take(&mut cur));
            w = 0;
        }
        cur.push(*pair);
        w += cw;
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    if chunks.is_empty() {
        chunks.push(word.to_vec());
    }
    chunks
}

fn chars_to_line(chars: &[StyledGlyph]) -> Line<'static> {
    if chars.is_empty() {
        return Line::from(String::new());
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut cur_style = chars[0].1;
    let mut buf = String::new();
    for (ch, st) in chars {
        if *st == cur_style {
            buf.push(*ch);
        } else {
            if !buf.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut buf), cur_style));
            }
            buf.push(*ch);
            cur_style = *st;
        }
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, cur_style));
    }
    Line::from(spans)
}

/// First span is a non-wrapping prefix (list bullet); body wraps with a matching indent on wraps.
#[must_use]
pub fn wrap_list_item_lines(lines: StyledLines, max_width: u16) -> StyledLines {
    let mw = max_width as usize;
    if mw == 0 || lines.is_empty() {
        return lines;
    }
    let mut it = lines.into_iter();
    let Some(first) = it.next() else {
        return Vec::new();
    };
    if first.spans.is_empty() {
        let mut rest: StyledLines = it.collect();
        rest.insert(0, first);
        return rest;
    }
    let prefix_w = first.spans[0].content.width();
    let budget = mw.saturating_sub(prefix_w);
    if budget == 0 {
        let mut out = vec![first];
        out.extend(it.flat_map(|ln| wrap_flow_block(vec![ln], max_width)));
        return out;
    }
    let rest = if first.spans.len() > 1 {
        Line::from(first.spans[1..].to_vec())
    } else {
        Line::from("")
    };
    let prefix_span = first.spans[0].clone();
    let wrapped = wrap_single_line(&rest, budget);
    let mut out: StyledLines = Vec::new();
    let indent = " ".repeat(prefix_w);
    for (i, wl) in wrapped.into_iter().enumerate() {
        if i == 0 {
            let mut spans = vec![prefix_span.clone()];
            spans.extend(wl.spans.iter().cloned());
            out.push(Line::from(spans));
        } else {
            let pad = Span::styled(indent.clone(), prefix_span.style);
            let mut spans = vec![pad];
            spans.extend(wl.spans.iter().cloned());
            out.push(Line::from(spans));
        }
    }
    out.extend(it.flat_map(|ln| wrap_flow_block(vec![ln], max_width)));
    out
}
