//! Plain-text word wrap (used by block quotes).

use ratatui::text::Line;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// `"> …"` blocks: wrap the quoted text; continuation lines use two leading spaces.
#[must_use]
pub fn wrap_quote_block(text: &str, max_width: u16) -> Vec<Line<'static>> {
    let mw = max_width as usize;
    let prefix = "> ";
    let pw = prefix.width();
    if mw <= pw + 1 {
        return text
            .lines()
            .map(|l| Line::from(format!("{prefix}{l}")))
            .collect();
    }
    let inner_w = mw - pw;
    let mut out: Vec<Line<'static>> = Vec::new();
    for para in text.lines() {
        if para.is_empty() {
            out.push(Line::from(prefix.to_string()));
            continue;
        }
        let wrapped = wrap_plain_words(para, inner_w);
        for (i, wline) in wrapped.into_iter().enumerate() {
            if i == 0 {
                out.push(Line::from(format!("{prefix}{wline}")));
            } else {
                out.push(Line::from(format!("  {wline}")));
            }
        }
    }
    out
}

fn wrap_plain_words(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for word in text.split_whitespace() {
        let ww = word.width();
        let gap = usize::from(!cur.is_empty());
        if !cur.is_empty() && cur_w + gap + ww > max_width {
            lines.push(std::mem::take(&mut cur));
            cur_w = 0;
        }
        if ww > max_width {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                cur_w = 0;
            }
            lines.extend(chunk_str(word, max_width));
            continue;
        }
        if !cur.is_empty() {
            cur.push(' ');
            cur_w += 1;
        }
        cur.push_str(word);
        cur_w += ww;
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn chunk_str(s: &str, max_width: usize) -> Vec<String> {
    let mut v = Vec::new();
    let mut cur = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > max_width && !cur.is_empty() {
            v.push(std::mem::take(&mut cur));
            w = 0;
        }
        cur.push(ch);
        w += cw;
    }
    if !cur.is_empty() {
        v.push(cur);
    }
    v
}
