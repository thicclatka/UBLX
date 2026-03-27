//! Main frame entry and tab bar. Snapshot/Delta content and search live in submodules.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::overlays;
use super::panes;

use crate::config::{LayoutOverlay, TOAST_CONFIG};
use crate::engine::db_ops::DuplicateGroup;
use crate::layout;
use crate::themes;
use crate::ui::{UI_CONSTANTS, UI_STRINGS};
use crate::utils::{format_timestamp_ns, toast_content_line_count};

/// Arguments for [`draw_ublx_frame`] that vary per frame (keeps arg count under clippy limit).
pub struct DrawFrameArgs<'a> {
    pub delta_data: Option<&'a layout::setup::DeltaViewData>,
    /// For snapshot mode pass `Some(all_rows)`; for delta/duplicates pass `None`.
    pub all_rows: Option<&'a [layout::setup::TuiRow]>,
    /// Snapshot mode: indexed dir (for mapping UBLX Settings path to "Local"/"Global" display).
    pub dir_to_ublx: Option<&'a std::path::Path>,
    /// Theme name (from opts); style functions use [`crate::themes::current`].
    pub theme_name: Option<&'a str>,
    /// Left/middle/right pane percentages (0–100). Hot-reloadable from config [layout].
    pub layout: &'a LayoutOverlay,
    /// Latest snapshot timestamp from `delta_log` (for categories panel footer). Set in Snapshot mode.
    pub latest_snapshot_ns: Option<i64>,
    /// When true, show dev-mode toast notifications.
    pub dev: bool,
    /// When non-empty, Duplicates tab is shown and this slice is the duplicate groups.
    pub duplicate_groups: Option<&'a [DuplicateGroup]>,
    /// When non-empty, Lenses tab is shown.
    pub lens_names: Option<&'a [String]>,
}

/// Main entry: layout and render main tabs, then Snapshot or Delta 3-pane content, search, help.
pub fn draw_ublx_frame(
    f: &mut Frame,
    state: &mut layout::setup::UblxState,
    view: &layout::setup::ViewData,
    right_content: &layout::setup::RightPaneContent,
    args: &DrawFrameArgs<'_>,
) {
    themes::set_current(Some(themes::theme_name_from_config(args.theme_name)));
    let area = f.area();

    draw_background(f, area, args);
    let (tabs_area, body_area) = split_tabs_and_body(area);
    draw_main_tabs(f, state, tabs_area, args);

    let body = compute_body_areas(body_area, args.layout);
    draw_main_content(f, state, view, right_content, args, &body);

    draw_toast_if_visible(f, state, args);
    if state.chrome.help_visible {
        overlays::render_help_box(f);
    }
    if state.theme.selector_visible {
        overlays::render_theme_selector(f, state.theme.selector_index);
    }
    draw_popups(f, state, &body, args);
    if let Some(ref sp) = state.startup_prompt {
        match &sp.phase {
            layout::setup::StartupPromptPhase::RootChoice {
                selected_index,
                roots,
            } => {
                let current = args
                    .dir_to_ublx
                    .unwrap_or_else(|| std::path::Path::new("."));
                overlays::popup::render_startup_welcome_root_choice(f, *selected_index, current, roots);
            }
            layout::setup::StartupPromptPhase::PreviousSettings { selected_index } => {
                overlays::popup::render_startup_previous_settings_prompt(f, *selected_index);
            }
            layout::setup::StartupPromptPhase::Enhance { selected_index } => {
                overlays::popup::render_startup_enhance_all_prompt(f, *selected_index);
            }
        }
    }
}

/// Render open menu, lens menu, space menu, and delete confirm popups when visible.
fn draw_popups(
    f: &mut Frame,
    state: &layout::setup::UblxState,
    body: &BodyAreas,
    args: &DrawFrameArgs<'_>,
) {
    let left = body.chunks[0];
    let middle = body.chunks[1];
    let content_sel = state.panels.content_state.selected().unwrap_or(0);
    let category_sel = state.panels.category_state.selected().unwrap_or(0);
    let in_snapshot_or_lenses = matches!(
        state.main_mode,
        layout::setup::MainMode::Snapshot | layout::setup::MainMode::Lenses
    );

    if state.open_menu.visible && in_snapshot_or_lenses {
        overlays::popup::render_open_menu(
            f,
            state.open_menu.selected_index,
            state.open_menu.can_terminal,
            middle,
            content_sel,
        );
    }
    if state.lens_menu.visible && state.lens_menu.name_input.is_none() && in_snapshot_or_lenses {
        let lens_names = args.lens_names.unwrap_or(&[]);
        overlays::popup::render_lens_menu(
            f,
            state.lens_menu.selected_index,
            middle,
            content_sel,
            lens_names,
        );
    }
    if state.enhance_policy_menu.visible && in_snapshot_or_lenses {
        overlays::popup::render_enhance_policy_menu(
            f,
            state.enhance_policy_menu.selected_index,
            middle,
            content_sel,
        );
    }
    if state.space_menu.visible
        && let Some(ref kind) = state.space_menu.kind
    {
        let (area, row) = match kind {
            layout::setup::SpaceMenuKind::FileActions { .. } => (middle, content_sel),
            layout::setup::SpaceMenuKind::LensPanelActions { .. } => (left, category_sel),
        };
        overlays::popup::render_context_menu(
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
        overlays::popup::render_delete_confirm(
            f,
            name,
            state.lens_confirm.delete_selected,
            left,
            category_sel,
        );
    }
}

fn draw_background(f: &mut Frame, area: Rect, _args: &DrawFrameArgs<'_>) {
    let bg = themes::current().background;
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

/// Shared path for Duplicates and Lenses: fullscreen right pane, or draw panes when data present, else placeholder.
fn draw_user_selected_mode_content<F>(
    f: &mut Frame,
    state: &mut layout::setup::UblxState,
    view: &layout::setup::ViewData,
    right_content: &layout::setup::RightPaneContent,
    body: &BodyAreas,
    has_data: bool,
    draw_panes: F,
) where
    F: FnOnce(
        &mut Frame,
        &mut layout::setup::UblxState,
        &layout::setup::ViewData,
        &layout::setup::RightPaneContent,
        &[Rect],
    ),
{
    let chunks = &body.chunks[..];
    if state.chrome.viewer_fullscreen {
        panes::draw_right_pane_fullscreen(f, state, right_content, view, body.main_area);
    } else if has_data {
        draw_panes(f, state, view, right_content, chunks);
    } else {
        panes::delta_mode::draw_delta_placeholder(f, chunks);
    }
}

fn draw_main_content(
    f: &mut Frame,
    state: &mut layout::setup::UblxState,
    view: &layout::setup::ViewData,
    right_content: &layout::setup::RightPaneContent,
    args: &DrawFrameArgs<'_>,
    body: &BodyAreas,
) {
    let chunks = &body.chunks[..];
    match state.main_mode {
        layout::setup::MainMode::Snapshot => {
            if state.chrome.viewer_fullscreen {
                panes::draw_right_pane_fullscreen(f, state, right_content, view, body.main_area);
            } else {
                panes::snapshot_mode::draw_categories_pane(f, state, view, chunks);
                panes::snapshot_mode::draw_contents_panel(
                    f,
                    state,
                    view,
                    args.all_rows,
                    args.dir_to_ublx,
                    chunks,
                );
                panes::draw_right_pane(f, state, right_content, chunks);
            }
        }
        layout::setup::MainMode::Delta => {
            if let Some(delta) = args.delta_data {
                panes::delta_mode::draw_delta_panes(
                    f,
                    panes::delta_mode::DrawDeltaPanesParams {
                        state,
                        delta,
                        view,
                        chunks,
                    },
                );
            } else {
                panes::delta_mode::draw_delta_placeholder(f, chunks);
            }
        }
        layout::setup::MainMode::Duplicates => draw_user_selected_mode_content(
            f,
            state,
            view,
            right_content,
            body,
            args.duplicate_groups.is_some_and(|g| !g.is_empty()),
            panes::draw_duplicates_panes,
        ),
        layout::setup::MainMode::Lenses => draw_user_selected_mode_content(
            f,
            state,
            view,
            right_content,
            body,
            args.lens_names.is_some_and(|n| !n.is_empty()),
            panes::draw_lenses_panes,
        ),
        layout::setup::MainMode::Settings => {
            if let Some(dir) = args.dir_to_ublx {
                panes::settings_mode::draw_settings_pane(f, body.main_area, state, dir);
            }
        }
    }
    if state.lens_menu.name_input.is_some() {
        let middle = body.chunks[1];
        let content_sel = state.panels.content_state.selected().unwrap_or(0);
        overlays::popup::render_lens_name_popup(
            f,
            middle,
            content_sel,
            state.lens_menu.name_input.as_deref().unwrap_or(""),
        );
    } else if let Some((_, ref input)) = state.lens_confirm.rename_input {
        overlays::popup::render_lens_rename_prompt(f, body.status_area, input);
    } else {
        draw_status_line(
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
        let content_lines = toast_content_line_count(slot);
        let max_h = TOAST_CONFIG.height_for(args.dev) as usize;
        let h = (TOAST_CONFIG.toast_height_offset as usize + content_lines)
            .clamp(TOAST_CONFIG.toast_height_min as usize, max_h) as u16;
        let h = h.min(area.height);
        let top = bottom.saturating_sub(h);
        if top >= area.y && h > 0 {
            overlays::render_toast_slot(f, Rect::new(x, top, w, h), slot);
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
        UI_STRINGS.main_tabs.snapshot,
        state.main_mode == layout::setup::MainMode::Snapshot,
    )
    .into_iter()
    .chain(layout::style::tab_node_segment(
        UI_STRINGS.main_tabs.delta,
        state.main_mode == layout::setup::MainMode::Delta,
    ))
    .chain(layout::style::tab_node_segment(
        UI_STRINGS.main_tabs.settings,
        state.main_mode == layout::setup::MainMode::Settings,
    ))
    .collect();
    if has_lenses {
        segments.extend(layout::style::tab_node_segment(
            UI_STRINGS.main_tabs.lenses,
            state.main_mode == layout::setup::MainMode::Lenses,
        ));
    }
    if has_duplicates {
        segments.extend(layout::style::tab_node_segment(
            UI_STRINGS.main_tabs.duplicates,
            state.main_mode == layout::setup::MainMode::Duplicates,
        ));
    }
    let line = Line::from(segments);
    f.render_widget(Paragraph::new(line), tabs_rect);
    f.render_widget(
        Paragraph::new(Line::from(ratatui::text::Span::styled(
            UI_STRINGS.brand.brand,
            layout::style::title_brand(),
        ))),
        brand_rect,
    );
}

/// Status line: powerline node (Latest Snapshot) + Search: + Esc to clear, all on one line
pub fn draw_status_line(
    f: &mut Frame,
    area: Rect,
    latest_snapshot_ns: Option<i64>,
    search_active: bool,
    search_query: &str,
) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if let Some(ns) = latest_snapshot_ns {
        let node_content = format!(
            "{}: {}",
            UI_STRINGS.search.latest_snapshot,
            format_timestamp_ns(ns)
        );
        spans.extend(layout::style::status_node_spans(&node_content));
    }
    if search_active || !search_query.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{}{}", UI_STRINGS.search.search_label, search_query),
            layout::style::search_text(),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            UI_STRINGS.search.esc_to_clear,
            layout::style::hint_text(),
        ));
    }
    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}
