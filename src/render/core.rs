//! Main frame entry and tab bar. Snapshot/Delta content and search live in submodules.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Paragraph};

use super::consts::UiStrings;
use super::delta;
use super::panels;
use super::right_pane;
use super::search;
use super::snapshot_panels;
use crate::config::TOAST_CONFIG;
use crate::layout::{help, setup, style, theme_selector, themes};
use crate::utils::notifications;

const UI: UiStrings = UiStrings::new();

/// Arguments for [draw_ublx_frame] that vary per frame (keeps arg count under clippy limit).
pub struct DrawFrameArgs<'a> {
    pub delta_data: Option<&'a setup::DeltaViewData>,
    /// For snapshot mode pass `Some(all_rows)` so contents panel resolves rows from indices; for delta pass `None`.
    pub all_rows: Option<&'a [setup::TuiRow]>,
    /// Snapshot mode: indexed dir (for mapping UBLX Settings path to "Local"/"Global" display).
    pub dir_to_ublx: Option<&'a std::path::Path>,
    /// Theme name (from opts); style functions use [crate::layout::themes::current].
    pub theme_name: Option<&'a str>,
    /// When true, skip painting app background so terminal default/transparency shows.
    pub transparent: bool,
    /// Latest snapshot timestamp from delta_log (for categories panel footer). Set in Snapshot mode.
    pub latest_snapshot_ns: Option<i64>,
    pub bumper: Option<&'a notifications::BumperBuffer>,
    pub dev: bool,
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
    draw_main_tabs(f, state, tabs_area);

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
        let vs = style::split_vertical(area, &[Constraint::Length(1), Constraint::Min(1)]);
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
                right_pane::draw_viewer_fullscreen(f, state, right, body.main_area);
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
                right_pane::draw_right_pane(f, state, right, right_rect);
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
    }
}

fn draw_toast_if_visible(f: &mut Frame, state: &setup::UblxState, args: &DrawFrameArgs<'_>) {
    let Some(_) = state.toast_visible_until else {
        return;
    };
    let Some(b) = args.bumper else {
        return;
    };
    let area = f.area();
    let w = TOAST_CONFIG.width_for(args.dev).min(area.width);
    let h = TOAST_CONFIG.height_for(args.dev).min(area.height);
    let x = area.x.saturating_add(
        area.width
            .saturating_sub(w)
            .saturating_sub(TOAST_CONFIG.hz_padding),
    );
    let y = area.y.saturating_add(
        area.height
            .saturating_sub(h)
            .saturating_sub(TOAST_CONFIG.vt_padding),
    );
    let toast_rect = Rect::new(x, y, w, h);
    notifications::render_toast(f, toast_rect, b, args.dev);
}

fn draw_main_tabs(f: &mut Frame, state: &setup::UblxState, area: Rect) {
    let outer = style::tab_row_padded(area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(4)])
        .split(outer[1]);
    let (tabs_rect, brand_rect) = (chunks[0], chunks[1]);
    let line = Line::from(
        style::tab_node_segment(
            UI.main_tab_snapshot,
            state.main_mode == setup::MainMode::Snapshot,
        )
        .into_iter()
        .chain(style::tab_node_segment(
            UI.main_tab_delta,
            state.main_mode == setup::MainMode::Delta,
        ))
        .collect::<Vec<_>>(),
    );
    f.render_widget(Paragraph::new(line), tabs_rect);
    f.render_widget(
        Paragraph::new(Line::from(ratatui::text::Span::styled(
            "UBLX",
            style::title_brand(),
        ))),
        brand_rect,
    );
}
