//! Snapshot mode: categories (left) and contents (middle) panels.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::ListItem;

use super::panes;

use crate::config::UblxPaths;
use crate::layout::setup;
use crate::layout::style;
use crate::ui::UI_STRINGS;

pub(super) fn draw_categories_panel(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    area: Rect,
) {
    let focused = matches!(state.panels.focus, setup::PanelFocus::Categories);
    let title = panes::set_title(UI_STRINGS.categories, focused);
    let mut items = vec![ListItem::new(UI_STRINGS.all_categories)];
    items.extend(
        view.filtered_categories
            .iter()
            .map(|s| ListItem::new(s.as_str())),
    );
    let block = panes::panel_block(title, focused);
    panes::draw_list_panel(
        f,
        items,
        block,
        state.panels.highlight_style,
        &mut state.panels.category_state,
        area,
    );
}

/// Map UBLX Settings path to "Local"/"Global" when dir_to_ublx is set; otherwise return path as-is.
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
        UI_STRINGS.local_config.to_string()
    } else if global.as_deref() == Some(path) {
        UI_STRINGS.global_config.to_string()
    } else {
        path.to_string()
    }
}

pub fn draw_contents_panel(
    f: &mut Frame,
    state: &mut setup::UblxState,
    view: &setup::ViewData,
    all_rows: Option<&[setup::TuiRow]>,
    dir_to_ublx: Option<&std::path::Path>,
    area: Rect,
) {
    let focused = matches!(state.panels.focus, setup::PanelFocus::Contents);
    let left_title = panes::set_title(UI_STRINGS.contents, focused);
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(if focused {
            style::panel_focused()
        } else {
            style::panel_unfocused()
        })
        .title(Line::from(left_title).left_aligned())
        .title_bottom(panes::line_for(
            state.panels.content_state.selected(),
            view.content_len,
        ));
    let items: Vec<ListItem> = if view.content_len == 0 {
        vec![ListItem::new(if state.search.query.is_empty() {
            UI_STRINGS.no_contents
        } else {
            UI_STRINGS.no_matches
        })]
    } else {
        view.iter_contents(all_rows)
            .map(|(path, category, _)| {
                let label = contents_display_label(path.as_str(), category.as_str(), dir_to_ublx);
                ListItem::new(label)
            })
            .collect()
    };
    panes::draw_list_panel(
        f,
        items,
        block,
        state.panels.highlight_style,
        &mut state.panels.content_state,
        area,
    );
}
