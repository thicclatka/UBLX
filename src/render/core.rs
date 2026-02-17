use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, Paragraph};

use super::consts::{UiStrings, panel_title};
use super::right_pane::{self, tab_node_segment};
use crate::config::TOAST_CONFIG;
use crate::layout::help::render_help_box;
use crate::layout::setup::{
    DeltaViewData, MainMode, PanelFocus, RightPaneContent, UblxState, ViewData,
};
use crate::layout::style;
use crate::utils::notifications;

const UI: UiStrings = UiStrings::new();

/// Main entry: layout and render main tabs, then Snapshot or Delta 3-pane content, search, help.
pub fn draw_ublx_frame(
    f: &mut Frame,
    state: &mut UblxState,
    view: &ViewData,
    right: &RightPaneContent,
    delta_data: Option<&DeltaViewData>,
    bumper: Option<&notifications::BumperBuffer>,
    dev: bool,
) {
    let area = f.area();
    let (tabs_area, body_area) = if area.height >= 2 {
        let vs = style::split_vertical(area, &[Constraint::Length(1), Constraint::Min(1)]);
        (vs[0], vs[1])
    } else {
        (area, area)
    };
    draw_main_tabs(f, state, tabs_area);

    let show_clear_hint = !state.search_query.is_empty() && !state.search_active;
    let (content_area, hint_area) = split_content_and_hint(body_area, show_clear_hint);
    let (main_area, search_area) = split_main_and_search(content_area, state.search_active);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ])
        .split(main_area);

    match state.main_mode {
        MainMode::Snapshot => {
            if state.viewer_fullscreen {
                right_pane::draw_viewer_fullscreen(f, state, right, main_area);
            } else {
                draw_categories_panel(f, state, view, chunks[0]);
                draw_contents_panel(f, state, view, chunks[1]);
                right_pane::draw_right_pane(f, state, right, chunks[2]);
            }
            if let Some(area) = search_area {
                draw_search_bar(f, state, area);
            }
        }
        MainMode::Delta => {
            if let Some(delta) = delta_data {
                draw_delta_panes(f, state, delta, chunks[0], chunks[1], chunks[2]);
            } else {
                draw_delta_placeholder(f, chunks[0], chunks[1], chunks[2]);
            }
        }
    }
    if let Some(rect) = hint_area {
        draw_search_clear_hint(f, state, rect);
    }
    if state.toast_visible_until.is_some()
        && let Some(b) = bumper
    {
        let area = f.area();
        let w = TOAST_CONFIG.width_for(dev).min(area.width);
        let h = TOAST_CONFIG.height_for(dev).min(area.height);
        let x = area.x + area.width.saturating_sub(w);
        let y = area.y + area.height.saturating_sub(h);
        let toast_rect = Rect::new(x, y, w, h);
        notifications::render_toast(f, toast_rect, b, dev);
    }
    if state.help_visible {
        render_help_box(f);
    }
}

fn draw_main_tabs(f: &mut Frame, state: &UblxState, area: Rect) {
    let outer = style::tab_row_padded(area);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(4)])
        .split(outer[1]);
    let (tabs_rect, brand_rect) = (chunks[0], chunks[1]);
    let line = Line::from(
        tab_node_segment(UI.main_tab_snapshot, state.main_mode == MainMode::Snapshot)
            .into_iter()
            .chain(tab_node_segment(UI.main_tab_delta, state.main_mode == MainMode::Delta))
            .collect::<Vec<_>>(),
    );
    f.render_widget(Paragraph::new(line), tabs_rect);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("UBLX", style::title_brand()))),
        brand_rect,
    );
}

fn draw_delta_placeholder(f: &mut Frame, left: Rect, middle: Rect, right: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Delta ");
    f.render_widget(Paragraph::new("Loading…").block(block), left);
    f.render_widget(
        Paragraph::new("—").block(Block::default().borders(Borders::ALL)),
        middle,
    );
    f.render_widget(
        Paragraph::new("—").block(
            Block::default()
                .borders(Borders::ALL)
                .title(UI.delta_right_title),
        ),
        right,
    );
}

fn draw_delta_panes(
    f: &mut Frame,
    state: &mut UblxState,
    delta: &DeltaViewData,
    left: Rect,
    middle: Rect,
    right: Rect,
) {
    let cat_idx = state.category_state.selected().unwrap_or(0).min(2);
    let focused = matches!(state.focus, PanelFocus::Categories);
    let labels: [(&'static str, Style); 3] = [
        (UI.delta_added, style::delta_added()),
        (UI.delta_mod, style::delta_mod()),
        (UI.delta_removed, style::delta_removed()),
    ];
    let items: Vec<ListItem> = labels
        .iter()
        .map(|(label, style)| {
            let span = Span::styled(*label, *style);
            ListItem::new(Line::from(span))
        })
        .collect();
    let title = panel_title("Delta type", focused);
    let left_block = panel_block(title, focused);
    f.render_stateful_widget(
        styled_list(items, left_block, focused, state.highlight_style),
        left,
        &mut state.category_state,
    );

    let paths = delta.paths_by_index(cat_idx);
    let content_focused = matches!(state.focus, PanelFocus::Contents);
    let mid_title = panel_title("Paths", content_focused);
    let mid_items: Vec<ListItem> = if paths.is_empty() {
        vec![ListItem::new("(none)")]
    } else {
        paths.iter().map(|p| ListItem::new(p.as_str())).collect()
    };
    draw_list_panel(
        f,
        mid_items,
        panel_block(mid_title, content_focused),
        content_focused,
        state.highlight_style,
        &mut state.content_state,
        middle,
    );

    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(UI.delta_right_title);
    f.render_widget(&right_block, right);
    let right_inner = right_block.inner(right);
    f.render_widget(
        Paragraph::new(Text::from(delta.overview_text.as_str())).scroll((state.preview_scroll, 0)),
        right_inner,
    );
}

fn split_content_and_hint(area: Rect, show_clear_hint: bool) -> (Rect, Option<Rect>) {
    if show_clear_hint && area.height >= 2 {
        let vs = style::split_vertical(area, &[Constraint::Min(1), Constraint::Length(1)]);
        (vs[0], Some(vs[1]))
    } else {
        (area, None)
    }
}

fn split_main_and_search(content_area: Rect, search_active: bool) -> (Rect, Option<Rect>) {
    let vertical = if search_active {
        style::split_vertical(content_area, &[Constraint::Min(1), Constraint::Length(3)])
    } else {
        style::split_vertical(content_area, &[Constraint::Min(1)])
    };
    let main_area = vertical[0];
    let search_area = if search_active {
        Some(vertical[1])
    } else {
        None
    };
    (main_area, search_area)
}

fn panel_block<'a, T: Into<Line<'a>>>(title: T, focused: bool) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title(title)
}

/// Build a list with standard panel styling (block, highlight, symbol, spacing).
fn styled_list<'a>(
    items: Vec<ListItem<'a>>,
    block: Block<'a>,
    focused: bool,
    highlight_style: ratatui::style::Style,
) -> List<'a> {
    let symbol = if focused {
        UI.list_highlight
    } else {
        UI.list_unfocused
    };
    List::new(items)
        .block(block)
        .highlight_style(highlight_style)
        .highlight_symbol(symbol)
        .highlight_spacing(HighlightSpacing::Always)
}

fn draw_list_panel(
    f: &mut Frame,
    items: Vec<ListItem>,
    block: Block,
    focused: bool,
    highlight_style: ratatui::style::Style,
    list_state: &mut ratatui::widgets::ListState,
    area: Rect,
) {
    f.render_stateful_widget(
        styled_list(items, block, focused, highlight_style),
        area,
        list_state,
    );
}

fn draw_categories_panel(f: &mut Frame, state: &mut UblxState, view: &ViewData, area: Rect) {
    let focused = matches!(state.focus, PanelFocus::Categories);
    let title = panel_title(UI.categories, focused);
    let mut items = vec![ListItem::new(UI.all_categories)];
    items.extend(
        view.filtered_categories
            .iter()
            .map(|s| ListItem::new(s.as_str())),
    );
    draw_list_panel(
        f,
        items,
        panel_block(title, focused),
        focused,
        state.highlight_style,
        &mut state.category_state,
        area,
    );
}

fn draw_contents_panel(f: &mut Frame, state: &mut UblxState, view: &ViewData, area: Rect) {
    let focused = matches!(state.focus, PanelFocus::Contents);
    let left_title = panel_title(UI.contents, focused);
    let current = state
        .content_state
        .selected()
        .map(|i| i + 1)
        .unwrap_or(0)
        .min(99_999);
    let total = view.content_len.min(99_999);
    let counter_str = format!("{:>5}/{:>5}", current, total);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title(Line::from(left_title).left_aligned())
        .title(Line::from(counter_str).right_aligned());
    let items: Vec<ListItem> = if view.filtered_contents_rows.is_empty() {
        vec![ListItem::new(if state.search_query.is_empty() {
            UI.no_contents
        } else {
            UI.no_matches
        })]
    } else {
        view.filtered_contents_rows
            .iter()
            .map(|(path, _, _, _)| ListItem::new(path.as_str()))
            .collect()
    };
    draw_list_panel(
        f,
        items,
        block,
        focused,
        state.highlight_style,
        &mut state.content_state,
        area,
    );
}

fn draw_search_bar(f: &mut Frame, state: &UblxState, area: Rect) {
    let search_line = format!("{}{}", UI.search_prompt, state.search_query);
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(style::search_border())
        .title(UI.search_title);
    f.render_widget(
        Paragraph::new(Text::from(search_line.as_str())).block(search_block),
        area,
    );
}

fn draw_search_clear_hint(f: &mut Frame, state: &UblxState, area: Rect) {
    let hint_text = format!("{}{}) ", UI.search_clear_hint_prefix, state.search_query);
    f.render_widget(Paragraph::new(hint_text).style(style::hint_text()), area);
}
