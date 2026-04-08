//! Conveyor-belt horizontal scroll for overflowing one-line labels (middle path list + dup/lens left list).

use std::path::Path;
use std::time::{Duration, Instant};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::LayoutOverlay;
use crate::layout::setup::{ContentMarqueeState, MainMode, PanelFocus, TuiRow, ViewData};
use crate::render::panes;

const MARQUEE_STEP: Duration = Duration::from_millis(110);
const MARQUEE_PAD: &str = "   ";

/// Shared per-tick inputs for marquee advancement
pub struct MarqueeTickCtx<'a> {
    pub focus: PanelFocus,
    pub main_mode: MainMode,
    pub viewer_fullscreen: bool,
    pub view: &'a ViewData,
    pub layout: &'a LayoutOverlay,
    pub term_width: u16,
    pub now: Instant,
}

/// Middle-list–specific rows for [`tick_content_marquee`].
pub struct ContentMarqueeTick<'a> {
    pub all_rows: Option<&'a [TuiRow]>,
    pub dir_to_ublx: Option<&'a Path>,
    pub content_selected: Option<usize>,
}

#[must_use]
pub fn left_pane_inner_width_cols(term_width: u16, layout: &LayoutOverlay) -> usize {
    let chunk_w = panes::three_pane_chunk_widths(term_width, layout)[0];
    panes::list_row_text_max_cols(chunk_w)
}

#[must_use]
pub fn middle_pane_inner_width_cols(term_width: u16, layout: &LayoutOverlay) -> usize {
    let chunk_w = panes::three_pane_chunk_widths(term_width, layout)[1];
    panes::list_row_text_max_cols(chunk_w)
}

/// Display string for one middle-pane row (paths list): snapshot settings labels or raw path.
#[must_use]
pub fn row_label_for_middle(
    main_mode: MainMode,
    view: &ViewData,
    all_rows: Option<&[TuiRow]>,
    dir_to_ublx: Option<&Path>,
    idx: usize,
) -> Option<String> {
    let row = view.row_at(idx, all_rows)?;
    let (path, cat, _) = row;
    match main_mode {
        MainMode::Snapshot => Some(crate::render::panes::snapshot_mode::contents_display_label(
            path.as_str(),
            cat.as_str(),
            dir_to_ublx,
        )),
        _ => Some(path.clone()),
    }
}

/// One visible line: `text` scrolled by `char_offset` into `text + pad`, clipped to `max_cols` display width.
#[must_use]
pub fn visible_line(text: &str, max_cols: usize, char_offset: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text) <= max_cols {
        return text.to_string();
    }
    let cycle: String = text.chars().chain(MARQUEE_PAD.chars()).collect();
    let cyc_len = cycle.chars().count();
    if cyc_len == 0 {
        return String::new();
    }
    let chars: Vec<char> = cycle.chars().collect();
    let mut off = char_offset % cyc_len;
    let mut out = String::new();
    let mut w = 0usize;
    let mut steps = 0usize;
    while w < max_cols && steps < cyc_len * 3 {
        let ch = chars[off % cyc_len];
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if cw > 0 && w + cw > max_cols {
            break;
        }
        out.push(ch);
        w += cw;
        off += 1;
        steps += 1;
    }
    out
}

fn cycle_char_len(text: &str) -> usize {
    text.chars().count() + MARQUEE_PAD.chars().count()
}

/// Middle path list: Snapshot / Delta / Duplicates / Lenses — when Contents focused and the selected row overflows.
pub fn tick_content_marquee(
    marquee: &mut ContentMarqueeState,
    ctx: &MarqueeTickCtx<'_>,
    content: &ContentMarqueeTick<'_>,
) {
    if ctx.viewer_fullscreen
        || ctx.main_mode == MainMode::Settings
        || ctx.view.content_len == 0
        || !matches!(
            ctx.main_mode,
            MainMode::Snapshot | MainMode::Delta | MainMode::Duplicates | MainMode::Lenses
        )
    {
        marquee.reset();
        return;
    }
    if !matches!(ctx.focus, PanelFocus::Contents) {
        marquee.reset();
        return;
    }

    let Some(global_idx) = content.content_selected else {
        marquee.reset();
        return;
    };
    let global_idx = global_idx.min(ctx.view.content_len.saturating_sub(1));

    let max_cols = middle_pane_inner_width_cols(ctx.term_width, ctx.layout);
    let Some(label) = row_label_for_middle(
        ctx.main_mode,
        ctx.view,
        content.all_rows,
        content.dir_to_ublx,
        global_idx,
    ) else {
        marquee.reset();
        return;
    };

    if UnicodeWidthStr::width(label.as_str()) <= max_cols {
        marquee.reset();
        return;
    }

    let key = (global_idx, label.clone());
    if marquee.anchor.as_ref() != Some(&key) {
        marquee.anchor = Some(key);
        marquee.offset = 0;
        marquee.last_advance = Some(ctx.now);
        return;
    }

    let cyc = cycle_char_len(label.as_str()).max(1);
    let last = marquee.last_advance.get_or_insert(ctx.now);
    if ctx.now.duration_since(*last) >= MARQUEE_STEP {
        marquee.offset = (marquee.offset + 1) % cyc;
        *last = ctx.now;
    }
}

/// Left category list: Duplicates / Lenses only — when Categories focused and the selected name overflows.
pub fn tick_category_marquee_dup_lens(
    marquee: &mut ContentMarqueeState,
    ctx: &MarqueeTickCtx<'_>,
    category_selected: Option<usize>,
) {
    if ctx.viewer_fullscreen || !matches!(ctx.main_mode, MainMode::Duplicates | MainMode::Lenses) {
        marquee.reset();
        return;
    }
    if ctx.view.filtered_categories.is_empty() {
        marquee.reset();
        return;
    }
    if !matches!(ctx.focus, PanelFocus::Categories) {
        marquee.reset();
        return;
    }

    let Some(global_idx) = category_selected else {
        marquee.reset();
        return;
    };
    let global_idx = global_idx.min(ctx.view.filtered_categories.len().saturating_sub(1));

    let max_cols = left_pane_inner_width_cols(ctx.term_width, ctx.layout);
    let label = ctx.view.filtered_categories[global_idx].as_str();

    if UnicodeWidthStr::width(label) <= max_cols {
        marquee.reset();
        return;
    }

    let key = (global_idx, label.to_string());
    if marquee.anchor.as_ref() != Some(&key) {
        marquee.anchor = Some(key);
        marquee.offset = 0;
        marquee.last_advance = Some(ctx.now);
        return;
    }

    let cyc = cycle_char_len(label).max(1);
    let last = marquee.last_advance.get_or_insert(ctx.now);
    if ctx.now.duration_since(*last) >= MARQUEE_STEP {
        marquee.offset = (marquee.offset + 1) % cyc;
        *last = ctx.now;
    }
}
