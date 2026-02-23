//! Main frame entry and tab bar. Snapshot/Delta content and search live in submodules.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use super::delta;
use super::duplicates;
use super::panels;
use super::search;
use super::snapshot_panels;
use crate::config::TOAST_CONFIG;
use crate::layout::{help, setup, style, theme_selector, themes};
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::notifications;

/// Arguments for [draw_ublx_frame] that vary per frame (keeps arg count under clippy limit).
pub struct DrawFrameArgs<'a> {
    pub delta_data: Option<&'a setup::DeltaViewData>,
    /// For snapshot mode pass `Some(all_rows)`; for delta/duplicates pass `None`.
    pub all_rows: Option<&'a [setup::TuiRow]>,
    /// Snapshot mode: indexed dir (for mapping UBLX Settings path to "Local"/"Global" display).
    pub dir_to_ublx: Option<&'a std::path::Path>,
    /// Theme name (from opts); style functions use [crate::layout::themes::current].
    pub theme_name: Option<&'a str>,
    /// When true, skip painting app background so terminal default/transparency shows.
    pub transparent: bool,
    /// Latest snapshot timestamp from delta_log (for categories panel footer). Set in Snapshot mode.
    pub latest_snapshot_ns: Option<i64>,
    /// When true, show dev-mode toast notifications.
    pub dev: bool,
    /// When non-empty, Duplicates tab is shown and this slice is the duplicate groups.
    pub duplicate_groups: Option<&'a [crate::engine::db_ops::DuplicateGroup]>,
    /// True while duplicate groups are being loaded in the background (show tab + "Loading…").
    pub duplicate_groups_loading: bool,
}

/// Main entry: layout and render main tabs, then Snapshot or Delta 3-pane content, search, help.
pub fn draw_ublx_frame(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right: &setup::RightPaneContent,
    args: &DrawFrameArgs<'_>,
) {
    themes::set_current(Some(themes::theme_name_from_config(args.theme_name)));
    let area = f.area();

    draw_background(f, area, args);
    let (tabs_area, body_area) = split_tabs_and_body(area);
    draw_main_tabs(
        f,
        state,
        tabs_area,
        args.duplicate_groups,
        args.duplicate_groups_loading,
    );

    let body = compute_body_areas(body_area);
    draw_main_content(f, state, view, right, args, &body);

    draw_toast_if_visible(f, state, args);
    if state.help_visible {
        help::render_help_box(f);
    }
    if state.theme_selector_visible {
        theme_selector::render_theme_selector(f, state.theme_selector_index);
    }
}

fn draw_background(f: &mut Frame, area: Rect, args: &DrawFrameArgs<'_>) {
    if args.transparent {
        return;
    }
    let bg = themes::current().background;
    f.render_widget(Block::default().style(Style::default().bg(bg)), area);
}

fn split_tabs_and_body(area: Rect) -> (Rect, Rect) {
    if area.height >= 2 {
        let vs = style::split_vertical(area, &UI_CONSTANTS.tab_row_constraints());
        (vs[0], vs[1])
    } else {
        (area, area)
    }
}

struct BodyAreas {
    main_area: Rect,
    status_area: Rect,
    chunks: Vec<Rect>,
}

fn compute_body_areas(body_area: Rect) -> BodyAreas {
    let (main_area, status_area) = panels::split_main_and_status(body_area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ])
        .split(main_area)
        .to_vec();
    BodyAreas {
        main_area,
        status_area,
        chunks,
    }
}

fn draw_main_content(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    right: &setup::RightPaneContent,
    args: &DrawFrameArgs<'_>,
    body: &BodyAreas,
) {
    let left = body.chunks[0];
    let middle = body.chunks[1];
    let right_rect = body.chunks[2];
    match state.main_mode {
        setup::MainMode::Snapshot => {
            if state.viewer_fullscreen {
                panels::draw_right_pane_fullscreen(f, state, right, body.main_area);
            } else {
                snapshot_panels::draw_categories_panel(f, state, view, left);
                snapshot_panels::draw_contents_panel(
                    f,
                    state,
                    view,
                    args.all_rows,
                    args.dir_to_ublx,
                    middle,
                );
                panels::draw_right_pane(f, state, right, right_rect);
            }
            search::draw_status_line(
                f,
                body.status_area,
                args.latest_snapshot_ns,
                state.search_active,
                &state.search_query,
            );
        }
        setup::MainMode::Delta => {
            if let Some(delta) = args.delta_data {
                delta::draw_delta_panes(
                    f,
                    delta::DrawDeltaPanesParams {
                        state,
                        delta,
                        view,
                        left,
                        middle,
                        right: right_rect,
                    },
                );
            } else {
                delta::draw_delta_placeholder(f, left, middle, right_rect);
            }
            search::draw_status_line(
                f,
                body.status_area,
                args.latest_snapshot_ns,
                state.search_active,
                &state.search_query,
            );
        }
        setup::MainMode::Duplicates => {
            if state.viewer_fullscreen {
                panels::draw_right_pane_fullscreen(f, state, right, body.main_area);
            } else if let Some(groups) = args.duplicate_groups
                && !groups.is_empty()
            {
                duplicates::draw_duplicates_panes(f, state, view, right, left, middle, right_rect);
            } else {
                delta::draw_delta_placeholder(f, left, middle, right_rect);
            }
            search::draw_status_line(
                f,
                body.status_area,
                args.latest_snapshot_ns,
                state.search_active,
                &state.search_query,
            );
        }
    }
}

fn draw_toast_if_visible(f: &mut Frame, state: &setup::UblxState, args: &DrawFrameArgs<'_>) {
    if state.toast_slots.is_empty() {
        return;
    }
    let area = f.area();
    let w = TOAST_CONFIG.width_for(args.dev).min(area.width);
    let x = area.x.saturating_add(
        area.width
            .saturating_sub(w)
            .saturating_sub(TOAST_CONFIG.hz_padding),
    );
    let gap = TOAST_CONFIG.toast_stack_gap;
    let mut bottom = area.y + area.height.saturating_sub(TOAST_CONFIG.vt_padding);
    for slot in state.toast_slots.iter().rev() {
        let h = TOAST_CONFIG
            .height_for_operation(args.dev, slot.operation.as_deref())
            .min(area.height);
        let top = bottom.saturating_sub(h);
        if top >= area.y && h > 0 {
            notifications::render_toast_slot(f, Rect::new(x, top, w, h), slot);
        }
        bottom = top.saturating_sub(gap);
    }
}

fn draw_main_tabs(
    f: &mut Frame,
    state: &setup::UblxState,
    area: Rect,
    duplicate_groups: Option<&[crate::engine::db_ops::DuplicateGroup]>,
    duplicate_groups_loading: bool,
) {
    let outer = style::tab_row_padded(area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(UI_CONSTANTS.brand_block_constraints())
        .split(outer[1]);
    let (tabs_rect, brand_rect) = (chunks[0], chunks[1]);
    let has_duplicates =
        duplicate_groups.is_some_and(|g| !g.is_empty()) || duplicate_groups_loading;
    let mut segments: Vec<_> = style::tab_node_segment(
        UI_STRINGS.main_tab_snapshot,
        state.main_mode == setup::MainMode::Snapshot,
    )
    .into_iter()
    .chain(style::tab_node_segment(
        UI_STRINGS.main_tab_delta,
        state.main_mode == setup::MainMode::Delta,
    ))
    .collect();
    if has_duplicates {
        segments.extend(style::tab_node_segment(
            UI_STRINGS.main_tab_duplicates,
            state.main_mode == setup::MainMode::Duplicates,
        ));
    }
    let line = Line::from(segments);
    f.render_widget(Paragraph::new(line), tabs_rect);
    f.render_widget(
        Paragraph::new(Line::from(ratatui::text::Span::styled(
            UI_STRINGS.brand,
            style::title_brand(),
        ))),
        brand_rect,
    );
}
