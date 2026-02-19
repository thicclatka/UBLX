//! Snapshot mode: categories (left) and contents (middle) panels.

use ratatui::Frame;
use ratatui::layout::{HorizontalAlignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use super::consts::{UiStrings, panel_title};
use super::panels;
use crate::config::UblxPaths;
use crate::layout::setup;
use crate::layout::style;

const UI: UiStrings = UiStrings::new();

pub(super) fn draw_categories_panel(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    area: Rect,
) {
    let focused = matches!(state.focus, setup::PanelFocus::Categories);
    let title = panel_title(UI.categories, focused);
    let mut items = vec![ListItem::new(UI.all_categories)];
    items.extend(
        view.filtered_categories
            .iter()
            .map(|s| ListItem::new(s.as_str())),
    );
    let block = panels::panel_block(title, focused);
    panels::draw_list_panel(
        f,
        items,
        block,
        focused,
        state.highlight_style,
        &mut state.category_state,
        area,
    );
}

/// Map UBLX Settings path to "Local"/"Global" when dir_to_ublx is set; otherwise return path as-is.
fn contents_display_label(
    path: &str,
    category: &str,
    dir_to_ublx: Option<&std::path::Path>,
) -> String {
    if category != "UBLX Settings" || dir_to_ublx.is_none() {
        return path.to_string();
    }
    let paths = UblxPaths::new(dir_to_ublx.unwrap());
    let local = paths
        .toml_path()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    let global = paths
        .global_config()
        .map(|p| p.to_string_lossy().into_owned());
    if local.as_deref() == Some(path) {
        UI.local_config.to_string()
    } else if global.as_deref() == Some(path) {
        UI.global_config.to_string()
    } else {
        path.to_string()
    }
}

pub(super) fn draw_contents_panel(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
    dir_to_ublx: Option<&std::path::Path>,
    area: Rect,
) {
    let focused = matches!(state.focus, setup::PanelFocus::Contents);
    let left_title = panel_title(UI.contents, focused);
    let current = state
        .content_state
        .selected()
        .map(|i| i + 1)
        .unwrap_or(0)
        .min(99_999);
    let total = view.content_len.min(99_999);
    let counter_str = format!("{:>5}/{:>5}", current, total);
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title(Line::from(left_title).left_aligned())
        .title(style::node_line(&counter_str, HorizontalAlignment::Right));
    let items: Vec<ListItem> = if view.content_len == 0 {
        vec![ListItem::new(if state.search_query.is_empty() {
            UI.no_contents
        } else {
            UI.no_matches
        })]
    } else {
        view.iter_contents(all_rows)
            .map(|(path, category, _)| {
                let label = contents_display_label(path.as_str(), category.as_str(), dir_to_ublx);
                ListItem::new(label)
            })
            .collect()
    };
    panels::draw_list_panel(
        f,
        items,
        block,
        focused,
        state.highlight_style,
        &mut state.content_state,
        area,
    );
}
