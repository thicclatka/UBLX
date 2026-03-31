//! Literal in-pane find (Ctrl+F): sync match ranges with right-pane text, scroll, highlighted draw.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratatui::text::{Line, Span, Text};

use crate::layout::{
    setup::{RightPaneContent, UblxState, ViewerFindState},
    style,
};
use crate::render::panes::{ensure_viewer_text_cache, viewer_find_haystack_text};

impl ViewerFindState {
    /// True when the trimmed find query has at least one character.
    #[inline]
    #[must_use]
    pub fn needle_nonempty(&self) -> bool {
        !self.query.trim().is_empty()
    }

    /// True when find should drive highlighting and scroll (committed or typing, with a non-empty needle).
    #[inline]
    #[must_use]
    pub fn find_affects_view(&self) -> bool {
        (self.active || self.committed) && self.needle_nonempty()
    }

    /// True when the find strip should show in the title bottom (any query text, or bar open, or committed).
    #[inline]
    #[must_use]
    pub fn title_bottom_visible(&self) -> bool {
        self.active || self.committed || self.needle_nonempty()
    }
}

/// True when `n` is `Some` and its trimmed text is non-empty (KV / optional needle for table cells).
#[inline]
#[must_use]
pub fn option_needle_nonempty(n: Option<&str>) -> bool {
    n.is_some_and(|s| !s.trim().is_empty())
}

/// All non-overlapping byte ranges of trimmed `needle` in `haystack` (public for integration tests).
#[inline]
#[must_use]
pub fn literal_match_ranges(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    let needle = needle.trim();
    if needle.is_empty() {
        return Vec::new();
    }
    haystack
        .match_indices(needle)
        .map(|(i, m)| (i, i + m.len()))
        .collect()
}

/// Line index (0-based) of the line containing `byte_off` (public for integration tests).
#[must_use]
pub fn line_byte_to_index(haystack: &str, byte_off: usize) -> u16 {
    haystack
        .get(..byte_off.min(haystack.len()))
        .map_or(0, |p| p.bytes().filter(|&b| b == b'\n').count()) as u16
}

pub fn scroll_preview_to_current(state: &mut UblxState, haystack: &str, viewport_h: u16) {
    let vf = &state.viewer_find;
    let Some(&(start, _)) = vf.ranges.get(vf.current) else {
        return;
    };
    let line = line_byte_to_index(haystack, start);
    let v = viewport_h.max(1);
    state.panels.preview_scroll = line.saturating_sub(v / 2);
}

/// Recompute match ranges when query or haystack changes; scroll to the current match.
pub fn sync(state: &mut UblxState, rc: &RightPaneContent, content_width: u16, viewport_h: u16) {
    let vf = &state.viewer_find;
    if !vf.needle_nonempty() || (!vf.active && !vf.committed) {
        state.viewer_find.ranges.clear();
        state.viewer_find.current = 0;
        state.viewer_find.last_sync_token = None;
        state.viewer_find.pending_scroll = false;
        return;
    }
    ensure_viewer_text_cache(state, rc, content_width);
    let haystack = viewer_find_haystack_text(state, rc, content_width);
    let mut h = DefaultHasher::new();
    state.viewer_find.query.hash(&mut h);
    haystack.hash(&mut h);
    let token = h.finish();
    if state.viewer_find.last_sync_token == Some(token) {
        if state.viewer_find.pending_scroll && !state.viewer_find.ranges.is_empty() {
            scroll_preview_to_current(state, &haystack, viewport_h);
            state.viewer_find.pending_scroll = false;
        }
        return;
    }
    state.viewer_find.last_sync_token = Some(token);
    state.viewer_find.ranges = literal_match_ranges(&haystack, &state.viewer_find.query);
    if state.viewer_find.current >= state.viewer_find.ranges.len() {
        state.viewer_find.current = state.viewer_find.ranges.len().saturating_sub(1);
    }
    state.viewer_find.pending_scroll = false;
    if !state.viewer_find.ranges.is_empty() {
        scroll_preview_to_current(state, &haystack, viewport_h);
    }
}

pub fn clear(state: &mut UblxState) {
    state.viewer_find = crate::layout::setup::ViewerFindState::default();
}

/// Highlight find ranges in a table cell segment; `global_start` is the byte offset of `text` in
/// the full haystack. Uses `viewer_find_match_table_cell` / `viewer_find_match_current` like the
/// main viewer body.
#[must_use]
pub fn highlight_table_cell_line(
    text: &str,
    global_start: usize,
    ranges: &[(usize, usize)],
    current_idx: usize,
) -> Line<'static> {
    line_to_spans(
        text,
        global_start,
        ranges,
        current_idx,
        style::text_style(),
        style::viewer_find_match_table_cell(),
        style::viewer_find_match_current_table_cell(),
    )
}

/// Highlight every literal occurrence of `needle` in `text` (trimmed). KV / sheet cells use
/// bold + underline (see `viewer_find_match_table_cell`).
#[must_use]
pub fn highlight_cell_line(text: &str, needle: &str) -> Line<'static> {
    let needle = needle.trim();
    let base_style = style::text_style();
    if needle.is_empty() {
        return Line::from(Span::styled(text.to_string(), base_style));
    }
    let hi = style::viewer_find_match_table_cell();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut pos = 0usize;
    for (idx, m) in text.match_indices(needle) {
        if idx > pos {
            spans.push(Span::styled(text[pos..idx].to_string(), base_style));
        }
        spans.push(Span::styled(m.to_string(), hi));
        pos = idx + m.len();
    }
    if pos < text.len() {
        spans.push(Span::styled(text[pos..].to_string(), base_style));
    }
    if spans.is_empty() {
        Line::from(Span::styled(text.to_string(), base_style))
    } else {
        Line::from(spans)
    }
}

/// Build [`Text`] with find highlights when there are match ranges.
#[must_use]
pub fn highlighted_body(state: &UblxState, haystack: &str) -> Text<'static> {
    let vf = &state.viewer_find;
    if vf.ranges.is_empty() || !vf.needle_nonempty() {
        return Text::raw(haystack.to_string());
    }
    let base_style = style::text_style();
    let hi = style::viewer_find_match();
    let cur = style::viewer_find_match_current();
    let mut lines_out: Vec<Line<'static>> = Vec::new();
    let mut line_base: usize = 0;
    for line in haystack.split('\n') {
        lines_out.push(line_to_spans(
            line, line_base, &vf.ranges, vf.current, base_style, hi, cur,
        ));
        line_base += line.len() + 1;
    }
    Text::from(lines_out)
}

fn line_to_spans(
    line: &str,
    base: usize,
    ranges: &[(usize, usize)],
    current_idx: usize,
    base_style: ratatui::style::Style,
    hi: ratatui::style::Style,
    cur: ratatui::style::Style,
) -> Line<'static> {
    let line_end = base.saturating_add(line.len());
    let mut local: Vec<(usize, usize, usize)> = Vec::new();
    for (mi, &(a, b)) in ranges.iter().enumerate() {
        if b <= base || a >= line_end {
            continue;
        }
        let la = a.saturating_sub(base).min(line.len());
        let lb = b.saturating_sub(base).min(line.len());
        if la < lb {
            local.push((la, lb, mi));
        }
    }
    if local.is_empty() {
        return Line::from(Span::styled(line.to_string(), base_style));
    }
    let mut cuts: Vec<usize> = vec![0, line.len()];
    for &(a, b, _) in &local {
        cuts.push(a);
        cuts.push(b);
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut i = 0usize;
    while i + 1 < cuts.len() {
        let s = cuts[i];
        let e = cuts[i + 1];
        if s < e {
            let piece = line.get(s..e).unwrap_or("").to_string();
            let st = if local
                .iter()
                .any(|&(a, b, mi)| mi == current_idx && a < e && b > s)
            {
                cur
            } else if local.iter().any(|&(a, b, _)| a < e && b > s) {
                hi
            } else {
                base_style
            };
            spans.push(Span::styled(piece, st));
        }
        i += 1;
    }
    Line::from(spans)
}
