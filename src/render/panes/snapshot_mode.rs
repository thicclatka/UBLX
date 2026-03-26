//! Snapshot mode: categories (left) and contents (middle) panels.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use super::middle::{CONTENTS_LIST_VIRTUALIZE_MIN, contents_list_viewport};

use crate::config::UblxPaths;
use crate::layout::setup::{self, ViewContents};
use crate::layout::style;
use crate::ui::UI_STRINGS;

/// Draw the categories (left) pane. `chunks` must have at least 1 element; uses `chunks[0]`.
pub fn draw_categories_pane(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    chunks: &[Rect],
) {
    let area = chunks[0];
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
    let title = super::set_title(UI_STRINGS.pane.categories, focused);
    let mut items = vec![ListItem::new(UI_STRINGS.list.all_categories)];
    items.extend(
        view.filtered_categories
            .iter()
            .map(|s| ListItem::new(s.as_str())),
    );
    let block = super::panel_block(title, focused);
    super::draw_list_panel(
        f,
        items,
        block,
        state.panels.highlight_style,
        focused,
        &mut state.panels.category_state,
        area,
    );
}

/// Map UBLX Settings path to "Local"/"Global" when `dir_to_ublx` is set; otherwise return path as-is.
fn contents_display_label(
    path: &str,
    category: &str,
    dir_to_ublx: Option<&std::path::Path>,
) -> String {
    if category != "UBLX Settings" {
        return path.to_string();
    }
    let Some(dir) = dir_to_ublx else {
        return path.to_string();
    };
    let paths = UblxPaths::new(dir);
    let local = paths
        .toml_path()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    let global = paths
        .global_config()
        .map(|p| p.to_string_lossy().into_owned());
    if local.as_deref() == Some(path) {
        UI_STRINGS.config.local.to_string()
    } else if global.as_deref() == Some(path) {
        UI_STRINGS.config.global.to_string()
    } else {
        path.to_string()
    }
}

/// Draw the contents (middle) panel. `chunks` must have at least 2 elements; uses `chunks[1]`.
pub fn draw_contents_panel(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
    dir_to_ublx: Option<&std::path::Path>,
    chunks: &[Rect],
) {
    let area = chunks[1];
    let focused = matches!(state.panels.focus, setup::PanelFocus::Contents);
    let left_title = super::set_title(UI_STRINGS.pane.contents, focused);
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title_style(style::panel_title_style(focused))
        .title(Line::from(left_title).left_aligned())
        .title_bottom(super::line_for(
            state.panels.content_state.selected(),
            view.content_len,
            state.main_mode,
            state.panels.content_sort,
        ));
    let total = view.content_len;

    let (items, window_start) = if total == 0 {
        (
            vec![ListItem::new(if state.search.query.is_empty() {
                UI_STRINGS.list.no_contents
            } else {
                UI_STRINGS.list.no_matches
            })],
            None,
        )
    } else if total >= CONTENTS_LIST_VIRTUALIZE_MIN
        && matches!(&view.contents, ViewContents::SnapshotIndices(_))
        && let Some(all_rows_slice) = all_rows
    {
        let inner_h = area.height.saturating_sub(2).max(1) as usize;
        let global_sel = state
            .panels
            .content_state
            .selected()
            .unwrap_or(0)
            .min(total - 1);
        let (w_start, w_end) = contents_list_viewport(total, global_sel, inner_h);
        let items = (w_start..w_end)
            .filter_map(|i| {
                view.row_at(i, Some(all_rows_slice))
                    .map(|(path, category, _)| {
                        ListItem::new(contents_display_label(
                            path.as_str(),
                            category.as_str(),
                            dir_to_ublx,
                        ))
                    })
            })
            .collect();
        (items, Some(w_start))
    } else {
        let items = view
            .iter_contents(all_rows)
            .map(|(path, category, _)| {
                ListItem::new(contents_display_label(
                    path.as_str(),
                    category.as_str(),
                    dir_to_ublx,
                ))
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

    super::draw_list_panel(
        f,
        items,
        block,
        state.panels.highlight_style,
        focused,
        &mut state.panels.content_state,
        area,
    );

    if window_start.is_some() {
        state.panels.content_state.select(saved_sel);
        *state.panels.content_state.offset_mut() = saved_off;
    }
}
