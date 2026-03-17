//! Main frame entry and tab bar. Snapshot/Delta content and search live in submodules.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use super::delta;
use super::duplicates;
use super::lenses;
use super::panes;
use super::search;
use super::snapshot_panels;

use crate::config::{LayoutOverlay, TOAST_CONFIG};
use crate::layout;
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::notifications;

/// Arguments for [draw_ublx_frame] that vary per frame (keeps arg count under clippy limit).
pub struct DrawFrameArgs<'a> {
    pub delta_data: Option<&'a layout::setup::DeltaViewData>,
    /// For snapshot mode pass `Some(all_rows)`; for delta/duplicates pass `None`.
    pub all_rows: Option<&'a [layout::setup::TuiRow]>,
    /// Snapshot mode: indexed dir (for mapping UBLX Settings path to "Local"/"Global" display).
    pub dir_to_ublx: Option<&'a std::path::Path>,
    /// Theme name (from opts); style functions use [crate::layout::themes::current].
    pub theme_name: Option<&'a str>,
    /// When true, skip painting app background so terminal default/transparency shows.
    pub transparent: bool,
    /// Left/middle/right pane percentages (0–100). Hot-reloadable from config [layout].
    pub layout: &'a LayoutOverlay,
    /// Latest snapshot timestamp from delta_log (for categories panel footer). Set in Snapshot mode.
    pub latest_snapshot_ns: Option<i64>,
    /// When true, show dev-mode toast notifications.
    pub dev: bool,
    /// When non-empty, Duplicates tab is shown and this slice is the duplicate groups.
    pub duplicate_groups: Option<&'a [crate::engine::db_ops::DuplicateGroup]>,
    /// When non-empty, Lenses tab is shown.
    pub lens_names: Option<&'a [String]>,
}

/// Main entry: layout and render main tabs, then Snapshot or Delta 3-pane content, search, help.
pub fn draw_ublx_frame(
    f: &mut Frame,
    state: &mut layout::setup::UblxState,
    view: &layout::setup::ViewData,
    right: &layout::setup::RightPaneContent,
    args: &DrawFrameArgs<'_>,
) {
    layout::themes::set_current(Some(layout::themes::theme_name_from_config(
        args.theme_name,
    )));
    let area = f.area();

    draw_background(f, area, args);
    let (tabs_area, body_area) = split_tabs_and_body(area);
    draw_main_tabs(f, state, tabs_area, args);

    let body = compute_body_areas(body_area, args.layout);
    draw_main_content(f, state, view, right, args, &body);

    draw_toast_if_visible(f, state, args);
    if state.help_visible {
        layout::help::render_help_box(f);
    }
    if state.theme.selector_visible {
        layout::theme_selector::render_theme_selector(f, state.theme.selector_index);
    }
    if state.open_menu.visible
        && matches!(
            state.main_mode,
            layout::setup::MainMode::Snapshot | layout::setup::MainMode::Lenses
        )
    {
        let middle = body.chunks[1];
        let content_sel = state.panels.content_state.selected().unwrap_or(0);
        layout::popup_menu::render_open_menu(
            f,
            state.open_menu.selected_index,
            state.open_menu.can_terminal,
            middle,
            content_sel,
        );
    }
    if state.lens_menu.visible
        && state.lens_menu.name_input.is_none()
        && matches!(
            state.main_mode,
            layout::setup::MainMode::Snapshot | layout::setup::MainMode::Lenses
        )
    {
        let middle = body.chunks[1];
        let content_sel = state.panels.content_state.selected().unwrap_or(0);
        let lens_names = args.lens_names.unwrap_or(&[]);
        layout::popup_menu::render_lens_menu(
            f,
            state.lens_menu.selected_index,
            middle,
            content_sel,
            lens_names,
        );
    }
    if state.space_menu.visible
        && let Some(ref kind) = state.space_menu.kind
    {
        let left = body.chunks[0];
        let middle = body.chunks[1];
        let content_sel = state.panels.content_state.selected().unwrap_or(0);
        let category_sel = state.panels.category_state.selected().unwrap_or(0);
        let (area, row) = match kind {
            layout::setup::SpaceMenuKind::FileActions { .. } => (middle, content_sel),
            layout::setup::SpaceMenuKind::LensPanelActions { .. } => (left, category_sel),
        };
        layout::popup_menu::render_space_menu(
            f,
            state.space_menu.selected_index,
            kind,
            state.main_mode,
            area,
            row,
        );
    }
    if state.lens_confirm.delete_visible
        && let Some(ref name) = state.lens_confirm.delete_lens_name
    {
        let left = body.chunks[0];
        let category_sel = state.panels.category_state.selected().unwrap_or(0);
        layout::popup_menu::render_delete_confirm(
            f,
            name,
            state.lens_confirm.delete_selected,
            left,
            category_sel,
        );
    }
}

fn draw_background(f: &mut Frame, area: Rect, args: &DrawFrameArgs<'_>) {
    if args.transparent {
        return;
    }
    let bg = layout::themes::current().background;
    f.render_widget(Block::default().style(Style::default().bg(bg)), area);
}

fn split_tabs_and_body(area: Rect) -> (Rect, Rect) {
    if area.height >= 2 {
        let vs = layout::style::split_vertical(area, &UI_CONSTANTS.tab_row_constraints());
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

fn compute_body_areas(body_area: Rect, layout: &LayoutOverlay) -> BodyAreas {
    let (main_area, status_area) = panes::split_main_and_status(body_area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout.left_pct),
            Constraint::Percentage(layout.middle_pct),
            Constraint::Percentage(layout.right_pct),
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
    state: &mut layout::setup::UblxState,
    view: &layout::setup::ViewData,
    right: &layout::setup::RightPaneContent,
    args: &DrawFrameArgs<'_>,
    body: &BodyAreas,
) {
    let left = body.chunks[0];
    let middle = body.chunks[1];
    let right_rect = body.chunks[2];
    match state.main_mode {
        layout::setup::MainMode::Snapshot => {
            if state.viewer_fullscreen {
                panes::draw_right_pane_fullscreen(f, state, right, body.main_area);
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
                panes::draw_right_pane(f, state, right, right_rect);
            }
        }
        layout::setup::MainMode::Delta => {
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
        }
        layout::setup::MainMode::Duplicates => {
            if state.viewer_fullscreen {
                panes::draw_right_pane_fullscreen(f, state, right, body.main_area);
            } else if let Some(groups) = args.duplicate_groups
                && !groups.is_empty()
            {
                duplicates::draw_duplicates_panes(f, state, view, right, left, middle, right_rect);
            } else {
                delta::draw_delta_placeholder(f, left, middle, right_rect);
            }
        }
        layout::setup::MainMode::Lenses => {
            if state.viewer_fullscreen {
                panes::draw_right_pane_fullscreen(f, state, right, body.main_area);
            } else if let Some(names) = args.lens_names
                && !names.is_empty()
            {
                lenses::draw_lenses_panes(f, state, view, right, left, middle, right_rect);
            } else {
                delta::draw_delta_placeholder(f, left, middle, right_rect);
            }
        }
    }
    if state.lens_menu.name_input.is_some() {
        layout::popup_menu::render_lens_name_prompt(
            f,
            body.status_area,
            state.lens_menu.name_input.as_deref().unwrap_or(""),
        );
    } else if let Some((_, ref input)) = state.lens_confirm.rename_input {
        layout::popup_menu::render_lens_rename_prompt(f, body.status_area, input);
    } else {
        search::draw_status_line(
            f,
            body.status_area,
            args.latest_snapshot_ns,
            state.search.active,
            &state.search.query,
        );
    }
}

fn draw_toast_if_visible(
    f: &mut Frame,
    state: &layout::setup::UblxState,
    args: &DrawFrameArgs<'_>,
) {
    if state.toasts.slots.is_empty() {
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
    for slot in state.toasts.slots.iter().rev() {
        let content_lines = notifications::toast_content_line_count(slot);
        let max_h = TOAST_CONFIG.height_for(args.dev) as usize;
        let h = (TOAST_CONFIG.toast_height_offset as usize + content_lines)
            .clamp(TOAST_CONFIG.toast_height_min as usize, max_h) as u16;
        let h = h.min(area.height);
        let top = bottom.saturating_sub(h);
        if top >= area.y && h > 0 {
            notifications::render_toast_slot(f, Rect::new(x, top, w, h), slot);
        }
        bottom = top.saturating_sub(gap);
    }
}

fn draw_main_tabs(
    f: &mut Frame,
    state: &layout::setup::UblxState,
    area: Rect,
    args: &DrawFrameArgs<'_>,
) {
    let outer = layout::style::tab_row_padded(area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(UI_CONSTANTS.brand_block_constraints())
        .split(outer[1]);
    let (tabs_rect, brand_rect) = (chunks[0], chunks[1]);
    let has_duplicates = args.duplicate_groups.is_some_and(|g| !g.is_empty());
    let has_lenses = args.lens_names.is_some_and(|n| !n.is_empty());
    let mut segments: Vec<_> = layout::style::tab_node_segment(
        UI_STRINGS.main_tab_snapshot,
        state.main_mode == layout::setup::MainMode::Snapshot,
    )
    .into_iter()
    .chain(layout::style::tab_node_segment(
        UI_STRINGS.main_tab_delta,
        state.main_mode == layout::setup::MainMode::Delta,
    ))
    .collect();
    if has_lenses {
        segments.extend(layout::style::tab_node_segment(
            UI_STRINGS.main_tab_lenses,
            state.main_mode == layout::setup::MainMode::Lenses,
        ));
    }
    if has_duplicates {
        segments.extend(layout::style::tab_node_segment(
            UI_STRINGS.main_tab_duplicates,
            state.main_mode == layout::setup::MainMode::Duplicates,
        ));
    }
    let line = Line::from(segments);
    f.render_widget(Paragraph::new(line), tabs_rect);
    f.render_widget(
        Paragraph::new(Line::from(ratatui::text::Span::styled(
            UI_STRINGS.brand,
            layout::style::title_brand(),
        ))),
        brand_rect,
    );
}
