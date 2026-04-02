//! Shared middle-pane drawing: paths list with selection counter at bottom.
//! Used by Delta, Duplicates, and Lenses modes.

use std::path::Path;

use ratatui::Frame;
use ratatui::layout::{HorizontalAlignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::ListItem;

use crate::layout::style::tab_active;
use crate::layout::{setup, style};
use crate::render::{marquee, panes};
use crate::themes;
use crate::ui::{UI_GLYPHS, UI_STRINGS, chord_chrome_active};

#[must_use]
pub fn contents_list_item_for_row(
    state: &setup::UblxState,
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
    dir_to_ublx: Option<&Path>,
    i: usize,
    global_sel: usize,
    max_cols: usize,
) -> ListItem<'static> {
    let path_label = marquee::row_label_for_middle(state.main_mode, view, all_rows, dir_to_ublx, i)
        .unwrap_or_default();
    let use_marquee = i == global_sel && matches!(state.panels.focus, setup::PanelFocus::Contents);

    if state.multiselect.active
        && let Some(row) = view.row_at(i, all_rows)
    {
        let selected = state.multiselect.selected.contains(&row.0);
        let path_cols = max_cols.saturating_sub(2);
        let path_visible = if use_marquee {
            marquee::visible_line(&path_label, path_cols, state.panels.content_marquee.offset)
        } else {
            path_label
        };
        let t = themes::current();
        let bg = t.tab_active_bg;
        let solid = Style::default().fg(bg).bg(bg);
        let line = if selected {
            Line::from(vec![
                Span::styled(UI_GLYPHS.swatch_block.to_string(), solid),
                Span::raw(" "),
                Span::raw(path_visible),
            ])
        } else {
            Line::from(vec![Span::raw("  "), Span::raw(path_visible)])
        };
        return ListItem::new(line);
    }

    let text = if use_marquee {
        marquee::visible_line(&path_label, max_cols, state.panels.content_marquee.offset)
    } else {
        path_label
    };
    ListItem::new(text)
}

/// Build full `ListItem` vecs only below this; larger lists use a viewport window.
pub const CONTENTS_LIST_VIRTUALIZE_MIN: usize = 512;
pub const CONTENTS_LIST_OVERSCAN_LINES: usize = 8;

/// Global content indices `[start, end)` (`end` exclusive) for one draw; keeps `global_sel` inside `[start, end)`.
#[must_use]
pub fn contents_list_viewport(total: usize, global_sel: usize, inner_h: usize) -> (usize, usize) {
    let inner_h = inner_h.max(1);
    let cap = (inner_h + CONTENTS_LIST_OVERSCAN_LINES * 2).min(total);
    let mut w_start = global_sel.saturating_sub(inner_h / 2 + CONTENTS_LIST_OVERSCAN_LINES);
    if w_start + cap > total {
        w_start = total.saturating_sub(cap);
    }
    let w_end = (w_start + cap).min(total);
    (w_start, w_end)
}

fn contents_list_can_virtualize(
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
) -> bool {
    match &view.contents {
        setup::ViewContents::DeltaRows(_) => true,
        setup::ViewContents::SnapshotIndices(_) => all_rows.is_some(),
    }
}

fn usize_digit_width(n: usize) -> usize {
    if n == 0 { 1 } else { n.ilog10() as usize + 1 }
}

/// Format `current/total` for the middle-pane counter; field width grows with the larger value.
#[must_use]
pub fn format_selection_counter(current: usize, total: usize) -> String {
    let w = usize_digit_width(current)
        .max(usize_digit_width(total))
        .max(1);
    format!("{current:>w$}/{total:>w$}")
}

/// Styled bottom line for the middle panel: current/total, right-aligned.
#[must_use]
pub fn counter_line(
    current: usize,
    total: usize,
    chord_mode: bool,
    transparent_page_chrome: bool,
) -> Line<'static> {
    style::node_line(
        &format_selection_counter(current, total),
        HorizontalAlignment::Right,
        chord_mode,
        transparent_page_chrome,
    )
}

/// Middle-pane sort node content text.
#[must_use]
pub fn sort_node_text(main_mode: setup::MainMode, sort: setup::ContentSort) -> Option<String> {
    let arrow = |dir: setup::SortDirection| match dir {
        setup::SortDirection::Asc => UI_GLYPHS.arrow_down.to_string(),
        setup::SortDirection::Desc => UI_GLYPHS.arrow_up.to_string(),
    };
    match main_mode {
        setup::MainMode::Snapshot | setup::MainMode::Duplicates => {
            let key = match sort.snapshot_key {
                setup::SnapshotSortKey::Name => "Name",
                setup::SnapshotSortKey::Size => "Size",
                setup::SnapshotSortKey::Mod => "Mod",
            };
            Some(format!("{key} {}", arrow(sort.snapshot_dir)))
        }
        setup::MainMode::Delta => Some(format!("Time {}", arrow(sort.delta_dir))),
        setup::MainMode::Lenses | setup::MainMode::Settings => None,
    }
}

/// Width (in terminal cells) of one powerline node for the provided content.
#[must_use]
pub fn node_display_width(content: &str) -> usize {
    // round-left + " {content} " + round-right
    content.len() + 4
}

/// Inputs for [`line_for`]: middle pane title-bottom counter and sort nodes.
#[derive(Clone, Copy)]
pub struct MiddlePaneFooterLineCtx {
    pub selected_index: Option<usize>,
    pub content_len: usize,
    pub main_mode: setup::MainMode,
    pub sort: setup::ContentSort,
    pub chord_mode: bool,
    pub multiselect_active: bool,
    pub multiselect_count: usize,
    pub transparent_page_chrome: bool,
}

/// Bottom line for the middle panel from list state: pass selected index (from `content_state.selected()`) and content length.
#[must_use]
pub fn line_for(ctx: MiddlePaneFooterLineCtx) -> Line<'static> {
    let current = ctx.selected_index.map_or(0, |i| i + 1);
    let total = ctx.content_len;
    let mut counter = format_selection_counter(current, total);
    if ctx.multiselect_active && ctx.multiselect_count > 0 {
        counter = format!("{counter} · {} sel", ctx.multiselect_count);
    }
    if let Some(sort_text) = sort_node_text(ctx.main_mode, ctx.sort) {
        style::viewer_footer_line(
            Some(&counter),
            None,
            Some(&sort_text),
            ctx.chord_mode,
            ctx.transparent_page_chrome,
        )
        .unwrap_or_else(|| {
            counter_line(current, total, ctx.chord_mode, ctx.transparent_page_chrome)
        })
    } else {
        counter_line(current, total, ctx.chord_mode, ctx.transparent_page_chrome)
    }
}

/// Draw the middle panel: list of paths (from view) with current/total counter at bottom.
/// Empty state shows "No contents" or "No matches" depending on search.
///
/// For [`setup::ViewContents::SnapshotIndices`], pass `Some(all_rows)` so rows resolve; for
/// [`setup::ViewContents::DeltaRows`], pass `None`. Pass `dir_to_ublx` for snapshot-style labels
/// (Duplicates/Lenses use path-only; `None` is fine).
pub fn draw_paths_list_with_counter(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
    dir_to_ublx: Option<&Path>,
    area: Rect,
    transparent_page_chrome: bool,
) {
    let content_focused = matches!(state.panels.focus, setup::PanelFocus::Contents);
    let mid_title = panes::panel_title_line(
        UI_STRINGS.paths.paths,
        content_focused,
        chord_chrome_active(&state.chrome),
    );
    let mid_block = panes::panel_block(mid_title, content_focused).title_bottom(
        panes::middle::line_for(MiddlePaneFooterLineCtx {
            selected_index: state.panels.content_state.selected(),
            content_len: view.content_len,
            main_mode: state.main_mode,
            sort: state.panels.content_sort,
            chord_mode: chord_chrome_active(&state.chrome),
            multiselect_active: state.multiselect.active,
            multiselect_count: state.multiselect.selected.len(),
            transparent_page_chrome,
        }),
    );
    let total = view.content_len;
    let global_sel = state
        .panels
        .content_state
        .selected()
        .unwrap_or(0)
        .min(total.saturating_sub(1));
    let max_cols = area.width.saturating_sub(2) as usize;

    let (mid_items, window_start) = if total == 0 {
        (
            vec![ListItem::new(if state.search.query.is_empty() {
                UI_STRINGS.list.no_contents
            } else {
                UI_STRINGS.list.no_matches
            })],
            None,
        )
    } else if total >= CONTENTS_LIST_VIRTUALIZE_MIN && contents_list_can_virtualize(view, all_rows)
    {
        let inner_h = area.height.saturating_sub(2).max(1) as usize;
        let (w_start, w_end) = contents_list_viewport(total, global_sel, inner_h);
        let items = (w_start..w_end)
            .map(|i| {
                contents_list_item_for_row(
                    state,
                    view,
                    all_rows,
                    dir_to_ublx,
                    i,
                    global_sel,
                    max_cols,
                )
            })
            .collect();
        (items, Some(w_start))
    } else {
        let items = (0..total)
            .map(|i| {
                contents_list_item_for_row(
                    state,
                    view,
                    all_rows,
                    dir_to_ublx,
                    i,
                    global_sel,
                    max_cols,
                )
            })
            .collect();
        (items, None)
    };

    let saved_sel = state.panels.content_state.selected();
    let saved_off = state.panels.content_state.offset();

    if let Some(ws) = window_start {
        let g = saved_sel.unwrap_or(0).min(total.saturating_sub(1));
        state
            .panels
            .content_state
            .select(Some(g.saturating_sub(ws)));
        *state.panels.content_state.offset_mut() = 0;
    }

    let highlight_style = if state.multiselect.active && content_focused {
        tab_active()
    } else {
        state.panels.highlight_style
    };
    panes::draw_list_panel(
        f,
        mid_items,
        mid_block,
        highlight_style,
        content_focused,
        &mut state.panels.content_state,
        area,
    );

    if window_start.is_some() {
        state.panels.content_state.select(saved_sel);
        *state.panels.content_state.offset_mut() = saved_off;
    }
}
