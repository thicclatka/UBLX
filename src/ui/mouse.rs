use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use unicode_width::UnicodeWidthStr;

use crate::config::LayoutOverlay;
use crate::layout::setup::{
    MainMode, PanelFocus, RightPaneContent, RightPaneMode, UblxState, ViewData,
};
use crate::render::{panes, viewers::image as viewer_image};
use crate::ui::MainTabFlags;
use crate::ui::consts::UI_CONSTANTS;
use crate::utils::{format_bytes, format_timestamp_ns};

#[derive(Clone, Copy)]
pub struct MouseContext<'a> {
    pub view: &'a ViewData,
    pub right_content: &'a RightPaneContent,
    pub frame_area: Rect,
    pub layout: &'a LayoutOverlay,
    pub tabs: MainTabFlags,
}

fn contains(area: Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

fn split_tabs_and_body(area: Rect) -> (Rect, Rect) {
    if area.height >= 2 {
        let vs = Layout::default()
            .direction(Direction::Vertical)
            .constraints(UI_CONSTANTS.tab_row_constraints())
            .split(area);
        (vs[0], vs[1])
    } else {
        (area, area)
    }
}

fn compute_main_chunks(body_area: Rect, layout: &LayoutOverlay) -> [Rect; 3] {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints(UI_CONSTANTS.status_line_constraints())
        .split(body_area);
    let main = vertical[0];
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(layout.left_pct),
            Constraint::Percentage(layout.middle_pct),
            Constraint::Percentage(layout.right_pct),
        ])
        .split(main);
    [chunks[0], chunks[1], chunks[2]]
}

fn click_to_list_index(area: Rect, y: u16, len: usize) -> Option<usize> {
    // List is inside bordered block: first content row at y+1.
    let first_row_y = area.y.saturating_add(1);
    if y < first_row_y {
        return None;
    }
    let idx = usize::from(y - first_row_y);
    (idx < len).then_some(idx)
}

fn click_to_labeled_tab_index(area: Rect, x: u16, labels: &[&str]) -> Option<usize> {
    if labels.is_empty() || !contains(area, x, area.y) {
        return None;
    }
    // Width mirrors style::tab_node_segment: round_left + " {label} " + round_right
    // => 1 + (label width + 2 spaces) + 1 = label width + 4
    let mut cursor = usize::from(area.x);
    let click_x = usize::from(x);
    for (idx, label) in labels.iter().enumerate() {
        let seg_w = UnicodeWidthStr::width(*label) + 4;
        if click_x >= cursor && click_x < cursor + seg_w {
            return Some(idx);
        }
        cursor += seg_w;
    }
    None
}

fn middle_sort_hit(area: Rect, x: u16, y: u16, state_ref: &UblxState, view_ref: &ViewData) -> bool {
    if area.height == 0 || area.width == 0 {
        return false;
    }
    let footer_y = area.y.saturating_add(area.height.saturating_sub(1));
    if y != footer_y {
        return false;
    }
    let Some(sort_text) = panes::sort_node_text(state_ref.main_mode, state_ref.panels.content_sort)
    else {
        return false;
    };
    let sort_w = panes::node_display_width(&sort_text);
    let counter = panes::format_selection_counter(
        state_ref
            .panels
            .content_state
            .selected()
            .map_or(0, |i| i + 1),
        view_ref.content_len,
    );
    let counter_w = panes::node_display_width(&counter);
    let total_w = sort_w.saturating_add(counter_w);
    let area_left = usize::from(area.x);
    let area_w = usize::from(area.width);
    let click_x = usize::from(x);
    if click_x < area_left || click_x >= area_left + area_w {
        return false;
    }
    let line_start = area_left + area_w.saturating_sub(total_w);
    let sort_start = line_start;
    let sort_end = sort_start.saturating_add(sort_w);
    click_x >= sort_start && click_x < sort_end
}

fn fullscreen_viewer_footer_width(
    state_mut: &mut UblxState,
    right_content_ref: &RightPaneContent,
) -> usize {
    if state_mut.right_pane_mode != RightPaneMode::Viewer {
        return 0;
    }
    viewer_image::sync_pdf_selection_state(state_mut, right_content_ref);
    let mut width = 0usize;
    if let Some(pdf) =
        viewer_image::pdf_page_footer_text(right_content_ref, &state_mut.viewer_image)
    {
        width = width.saturating_add(panes::node_display_width(&pdf));
    }
    if let Some(size) = right_content_ref.viewer_byte_size {
        width = width.saturating_add(panes::node_display_width(&format_bytes(size)));
    }
    if let Some(ns) = right_content_ref.viewer_mtime_ns {
        width = width.saturating_add(panes::node_display_width(&format_timestamp_ns(ns)));
    }
    width
}

fn fullscreen_sort_hit(
    area: Rect,
    x: u16,
    y: u16,
    state_mut: &mut UblxState,
    view_ref: &ViewData,
    right_content_ref: &RightPaneContent,
) -> bool {
    if area.height == 0 || area.width == 0 {
        return false;
    }
    let footer_y = area.y.saturating_add(area.height.saturating_sub(1));
    if y != footer_y {
        return false;
    }
    let Some(sort_text) = panes::sort_node_text(state_mut.main_mode, state_mut.panels.content_sort)
    else {
        return false;
    };
    let sort_w = panes::node_display_width(&sort_text);
    let counter = panes::format_selection_counter(
        state_mut
            .panels
            .content_state
            .selected()
            .map_or(0, |i| i + 1),
        view_ref.content_len,
    );
    let counter_w = panes::node_display_width(&counter);
    let trailer_w = fullscreen_viewer_footer_width(state_mut, right_content_ref);
    let total_w = sort_w.saturating_add(counter_w).saturating_add(trailer_w);
    let area_left = usize::from(area.x);
    let area_w = usize::from(area.width);
    let click_x = usize::from(x);
    if click_x < area_left || click_x >= area_left + area_w {
        return false;
    }
    let line_start = area_left + area_w.saturating_sub(total_w);
    let sort_start = line_start;
    let sort_end = sort_start.saturating_add(sort_w);
    click_x >= sort_start && click_x < sort_end
}

fn cycle_sort_from_mouse(state_mut: &mut UblxState, right_content_ref: &RightPaneContent) {
    state_mut
        .panels
        .sort_anchor_path
        .clone_from(&right_content_ref.viewer_path);
    state_mut.panels.content_sort = state_mut
        .panels
        .content_sort
        .cycle_for_mode(state_mut.main_mode);
}

fn rough_wrapped_line_count(text: &str, width: u16) -> usize {
    let w = usize::from(width.max(1));
    text.lines()
        .map(|line| {
            let chars = line.chars().count();
            chars.div_ceil(w).max(1)
        })
        .sum::<usize>()
        .max(1)
}

fn estimate_total_lines(
    state_ref: &UblxState,
    right_content_ref: &RightPaneContent,
    text_width: u16,
) -> usize {
    match state_ref.right_pane_mode {
        RightPaneMode::Viewer => right_content_ref
            .viewer
            .as_deref()
            .map_or(1, |s| rough_wrapped_line_count(s, text_width)),
        RightPaneMode::Templates => {
            rough_wrapped_line_count(&right_content_ref.templates, text_width)
        }
        RightPaneMode::Metadata => right_content_ref
            .metadata
            .as_deref()
            .map_or(1, |s| rough_wrapped_line_count(s, text_width)),
        RightPaneMode::Writing => right_content_ref
            .writing
            .as_deref()
            .map_or(1, |s| rough_wrapped_line_count(s, text_width)),
    }
}

struct MouseFrameAreas {
    tabs_click_rect: Rect,
    fullscreen_main_area: Rect,
    left: Rect,
    middle: Rect,
    right: Rect,
}

fn compute_mouse_frame_areas(frame_area: Rect, layout: &LayoutOverlay) -> MouseFrameAreas {
    let (tabs_area, body_area) = split_tabs_and_body(frame_area);
    let body_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints(UI_CONSTANTS.status_line_constraints())
        .split(body_area);
    let fullscreen_main_area = body_vertical[0];
    let [left, middle, right] = compute_main_chunks(body_area, layout);
    // Main tabs row geometry mirrors render::draw_main_tabs:
    // tab_row_padded -> split tabs vs brand -> tabs rect is first chunk.
    let tab_outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(UI_CONSTANTS.h_pad),
            Constraint::Min(0),
            Constraint::Length(UI_CONSTANTS.h_pad),
        ])
        .split(tabs_area);
    let tab_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(UI_CONSTANTS.brand_block_constraints())
        .split(tab_outer[1]);
    let tabs_click_rect = tab_chunks[0];

    MouseFrameAreas {
        tabs_click_rect,
        fullscreen_main_area,
        left,
        middle,
        right,
    }
}

fn mouse_left_down_right_pane(
    state_mut: &mut UblxState,
    x: u16,
    y: u16,
    right: Rect,
    right_content_ref: &RightPaneContent,
) -> bool {
    if !contains(right, x, y) {
        return false;
    }
    let right_inner = Rect {
        x: right.x.saturating_add(1),
        y: right.y.saturating_add(1),
        width: right.width.saturating_sub(2),
        height: right.height.saturating_sub(2),
    };
    let right_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(right_inner);
    let right_tab_outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(UI_CONSTANTS.h_pad),
            Constraint::Min(0),
            Constraint::Length(UI_CONSTANTS.h_pad),
        ])
        .split(right_split[0]);
    let right_tab_rect = right_tab_outer[1];
    if contains(right_tab_rect, x, y) {
        let mut tabs = vec![RightPaneMode::Viewer, RightPaneMode::Templates];
        if right_content_ref.metadata.is_some() {
            tabs.push(RightPaneMode::Metadata);
        }
        if right_content_ref.writing.is_some() {
            tabs.push(RightPaneMode::Writing);
        }
        let labels: Vec<&str> = tabs
            .iter()
            .map(|m| match m {
                RightPaneMode::Viewer => "Viewer",
                RightPaneMode::Templates => "Templates",
                RightPaneMode::Metadata => "Metadata",
                RightPaneMode::Writing => "Writing",
            })
            .collect();
        if let Some(idx) = click_to_labeled_tab_index(right_tab_rect, x, &labels) {
            state_mut.right_pane_mode = tabs[idx];
            return true;
        }
    }

    if state_mut.right_pane_mode == RightPaneMode::Viewer && right_inner.width > 0 {
        let content_outer = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(UI_CONSTANTS.h_pad),
                Constraint::Min(0),
                Constraint::Length(UI_CONSTANTS.h_pad),
            ])
            .split(right_split[2]);
        let content_rect = content_outer[1];
        let viewport_h = content_rect.height.saturating_sub(UI_CONSTANTS.v_pad);
        if content_rect.width > 0 && viewport_h > 0 {
            let text_width = content_rect.width.saturating_sub(1).max(1);
            let total_lines = estimate_total_lines(state_mut, right_content_ref, text_width);
            if total_lines <= usize::from(viewport_h) {
                return true;
            }
            let max_scroll = total_lines.saturating_sub(usize::from(viewport_h)) as u16;
            let scrollbar_x = content_rect
                .x
                .saturating_add(content_rect.width.saturating_sub(1));
            if x == scrollbar_x && contains(content_rect, x, y) {
                let track_top = content_rect.y;
                let rel = y
                    .saturating_sub(track_top)
                    .min(viewport_h.saturating_sub(1));
                let denom = viewport_h.saturating_sub(1).max(1);
                state_mut.panels.preview_scroll =
                    ((u32::from(rel) * u32::from(max_scroll)) / u32::from(denom)) as u16;
                return true;
            }
        }
    }
    false
}

pub fn handle_mouse_event(
    state_mut: &mut UblxState,
    event: MouseEvent,
    ctx: MouseContext<'_>,
) -> bool {
    let MouseContext {
        view: view_ref,
        right_content: right_content_ref,
        frame_area,
        layout,
        tabs,
    } = ctx;
    // Keep first pass conservative: no mouse interaction while modals are open.
    if state_mut.theme.selector_visible
        || state_mut.chrome.help_visible
        || state_mut.open_menu.visible
        || state_mut.lens_menu.visible
        || state_mut.space_menu.visible
        || state_mut.enhance_policy_menu.visible
        || state_mut.lens_confirm.delete_visible
        || state_mut.startup_prompt.is_some()
    {
        return false;
    }

    let x = event.column;
    let y = event.row;
    let areas = compute_mouse_frame_areas(frame_area, layout);

    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if state_mut.chrome.viewer_fullscreen
                && fullscreen_sort_hit(
                    areas.fullscreen_main_area,
                    x,
                    y,
                    state_mut,
                    view_ref,
                    right_content_ref,
                )
            {
                cycle_sort_from_mouse(state_mut, right_content_ref);
                return true;
            }
            if contains(areas.tabs_click_rect, x, y) {
                let mut main_tabs = vec![MainMode::Snapshot, MainMode::Delta, MainMode::Settings];
                if tabs.has_lenses {
                    main_tabs.push(MainMode::Lenses);
                }
                if tabs.has_duplicates {
                    main_tabs.push(MainMode::Duplicates);
                }
                let labels: Vec<&str> = main_tabs
                    .iter()
                    .map(|m| match m {
                        MainMode::Snapshot => "Snapshot",
                        MainMode::Delta => "Delta",
                        MainMode::Settings => "Settings",
                        MainMode::Lenses => "Lenses",
                        MainMode::Duplicates => "Duplicates",
                    })
                    .collect();
                if let Some(idx) = click_to_labeled_tab_index(areas.tabs_click_rect, x, &labels) {
                    state_mut.main_mode = main_tabs[idx];
                    return true;
                }
            }

            if contains(areas.left, x, y) {
                state_mut.panels.focus = PanelFocus::Categories;
                if let Some(idx) = click_to_list_index(areas.left, y, view_ref.category_list_len) {
                    state_mut.panels.category_state.select(Some(idx));
                }
                return true;
            }

            if contains(areas.middle, x, y) {
                if middle_sort_hit(areas.middle, x, y, state_mut, view_ref) {
                    cycle_sort_from_mouse(state_mut, right_content_ref);
                    return true;
                }
                state_mut.panels.focus = PanelFocus::Contents;
                if let Some(idx) = click_to_list_index(areas.middle, y, view_ref.content_len) {
                    state_mut.panels.content_state.select(Some(idx));
                }
                return true;
            }

            if mouse_left_down_right_pane(state_mut, x, y, areas.right, right_content_ref) {
                return true;
            }
        }
        MouseEventKind::ScrollUp => {
            if contains(areas.right, x, y) {
                state_mut.panels.preview_scroll = state_mut.panels.preview_scroll.saturating_sub(3);
                return true;
            }
        }
        MouseEventKind::ScrollDown => {
            if contains(areas.right, x, y) {
                state_mut.panels.preview_scroll = state_mut.panels.preview_scroll.saturating_add(3);
                return true;
            }
        }
        _ => {}
    }
    false
}
