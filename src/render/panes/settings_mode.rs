//! Settings tab: two panes (controls + raw file); scope tabs (powerline) on the left.

use std::path::Path;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::config::{UblxOverlay, UblxPaths, load_ublx_toml};
use crate::handlers::applets::settings;
use crate::layout::setup::{SettingsConfigScope, UblxState};
use crate::layout::style;
use crate::render::{path_lines, scrollable_content};
use crate::ui::{UI_CONSTANTS, UI_GLYPHS, UI_STRINGS};
use crate::utils;

fn row_prefix(active: bool) -> &'static str {
    if active {
        UI_GLYPHS.settings_row_active
    } else {
        UI_GLYPHS.indent_two_spaces
    }
}

/// Row label: dimmed (hint) when inherited-only and not focused; bold when active.
fn label_style(active: bool, dimmed: bool) -> Style {
    if dimmed && !active {
        style::hint_text()
    } else if active {
        style::text_style().add_modifier(Modifier::BOLD)
    } else {
        style::text_style()
    }
}

/// "Edit layout …" line: active row uses tab or hint bold; inactive uses hint vs body by dimmed.
fn layout_edit_line_style(active: bool, dimmed: bool) -> Style {
    if active {
        if dimmed {
            style::hint_text().add_modifier(Modifier::BOLD)
        } else {
            style::tab_active().add_modifier(Modifier::BOLD)
        }
    } else if dimmed {
        style::hint_text()
    } else {
        style::text_style()
    }
}

/// Layout percent value cell: active → tab highlight; dimmed inactive → hint.
fn layout_value_style(active: bool, dimmed: bool) -> Style {
    if dimmed && !active {
        style::hint_text()
    } else if active {
        style::tab_active()
    } else {
        style::text_style()
    }
}

fn scope_tab_spans(
    scope: SettingsConfigScope,
    global_label: &'static str,
    local_label: &'static str,
) -> Vec<Span<'static>> {
    let mut scope_spans: Vec<Span<'static>> =
        style::tab_node_segment(global_label, scope == SettingsConfigScope::Global);
    scope_spans.extend(style::tab_node_segment(
        local_label,
        scope == SettingsConfigScope::Local,
    ));
    scope_spans
}

fn push_scope_path_header(
    left_lines: &mut Vec<Line>,
    scope: SettingsConfigScope,
    global_path_str: &str,
    local_path_str: &str,
    path_wrap: usize,
) {
    match scope {
        SettingsConfigScope::Global => {
            left_lines.extend(path_lines::wrap_lines_at_path_separators(
                global_path_str,
                path_wrap,
                UI_GLYPHS.indent_two_spaces,
                style::hint_text(),
            ));
            left_lines.push(Line::from(Span::styled(
                "BE CAREFUL: CHANGING GLOBAL SETTINGS",
                style::delta_removed().add_modifier(Modifier::BOLD),
            )));
            left_lines.push(Line::from(Span::styled(
                "Any change here affects values not set in local",
                style::hint_text(),
            )));
        }
        SettingsConfigScope::Local => {
            left_lines.extend(path_lines::wrap_lines_at_path_separators(
                local_path_str,
                path_wrap,
                UI_GLYPHS.indent_two_spaces,
                style::hint_text(),
            ));
        }
    }
}

fn push_bool_rows(
    left_lines: &mut Vec<Line>,
    scope: SettingsConfigScope,
    n_bool: usize,
    cur: usize,
    local_ctx: Option<&(Option<UblxOverlay>, UblxOverlay)>,
    overlay: Option<&UblxOverlay>,
) {
    for i in 0..n_bool {
        let (v, dimmed) = if let Some((local_o, merged)) = local_ctx {
            (
                settings::overlay_bool(merged, SettingsConfigScope::Local, i),
                !settings::local_bool_is_explicit(local_o.as_ref(), i),
            )
        } else {
            (
                overlay.is_some_and(|o| settings::overlay_bool(o, scope, i)),
                false,
            )
        };
        let row_active = cur == i;
        let label_st = label_style(row_active, dimmed);
        let mut spans = vec![Span::styled(
            format!(
                "{}{}: ",
                row_prefix(row_active),
                settings::bool_row_label(scope, i, true)
            ),
            label_st,
        )];
        spans.push(yn_cell(true, v, dimmed));
        spans.push(Span::raw(" "));
        spans.push(yn_cell(false, v, dimmed));
        left_lines.push(Line::from(spans));
    }
}

fn push_layout_edit_section(
    left_lines: &mut Vec<Line>,
    state: &UblxState,
    n_bool: usize,
    layout_dimmed: bool,
) {
    let btn = n_bool;
    let cur = state.settings.left_cursor;
    let layout_btn_active = cur == btn;
    let edit_primary = if state.settings.layout_unlocked {
        "Enter to save and lock"
    } else {
        "Enter to unlock"
    };
    let edit_line_st = layout_edit_line_style(layout_btn_active, layout_dimmed);
    left_lines.push(Line::from(vec![Span::styled(
        format!(
            "{}Edit layout ({edit_primary})",
            row_prefix(layout_btn_active)
        ),
        edit_line_st,
    )]));

    if state.settings.layout_unlocked {
        for (fi, buf) in [
            (0usize, state.settings.layout_left_buf.as_str()),
            (1, state.settings.layout_mid_buf.as_str()),
            (2, state.settings.layout_right_buf.as_str()),
        ] {
            let field_cur = btn + 1 + fi;
            let active = cur == field_cur;
            let label = match fi {
                0 => "left_pct ",
                1 => "middle_pct ",
                _ => "right_pct ",
            };
            let label_st = label_style(active, layout_dimmed);
            let val_st = layout_value_style(active, layout_dimmed);
            left_lines.push(Line::from(vec![
                Span::styled(format!("{}{label}", row_prefix(active)), label_st),
                Span::styled(
                    if buf.is_empty() {
                        " ".to_string()
                    } else {
                        buf.to_string()
                    },
                    val_st,
                ),
            ]));
        }
    }
}

/// `FFmpeg` + PDF raster backends (same binaries as video / PDF preview).
fn push_external_apps_section(left_lines: &mut Vec<Line>) {
    left_lines.push(Line::from(""));
    left_lines.push(Line::from(Span::styled(
        "External apps",
        style::hint_text().add_modifier(Modifier::BOLD),
    )));
    let ffmpeg_ok = crate::utils::ffmpeg_available();
    left_lines.push(Line::from(vec![
        Span::styled(
            format!("{}FFmpeg: ", UI_GLYPHS.indent_two_spaces),
            style::text_style(),
        ),
        Span::styled(
            if ffmpeg_ok { "available" } else { "not found" },
            if ffmpeg_ok {
                style::tab_active()
            } else {
                style::hint_text()
            },
        ),
    ]));
    let pop = utils::poppler_pdftoppm_available();
    let mu = utils::mutool_available();
    let pdf_detail: &'static str = match (pop, mu) {
        (true, true) => "Poppler (pdftoppm) · MuPDF (mutool)",
        (true, false) => "Poppler (pdftoppm) only",
        (false, true) => "MuPDF (mutool) only",
        (false, false) => "not found",
    };
    let pdf_st = if pop || mu {
        style::tab_active()
    } else {
        style::hint_text()
    };
    left_lines.push(Line::from(vec![
        Span::styled(
            format!("{}PDF: ", UI_GLYPHS.indent_two_spaces),
            style::text_style(),
        ),
        Span::styled(pdf_detail, pdf_st),
    ]));
    left_lines.push(Line::from(""));
    left_lines.push(Line::from(""));
    left_lines.push(Line::from(Span::styled(
        "* — settings applied on next snapshot",
        style::hint_text(),
    )));
}

fn render_settings_toml_preview(
    f: &mut Frame,
    right_inner: Rect,
    state: &mut UblxState,
    paths: &UblxPaths,
    scope: SettingsConfigScope,
) {
    let toml_text = settings::resolve_config_path(paths, scope)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    let lines: Vec<&str> = toml_text.lines().collect();
    let total_lines = lines.len().max(1);
    let layout = scrollable_content::layout_scrollable_content(
        right_inner,
        total_lines,
        &mut state.settings.right_scroll,
        UI_CONSTANTS.v_pad,
    );
    let start = layout.scroll_y as usize;
    let view_h = layout.content_rect.height as usize;
    let visible: Vec<Line> = lines
        .iter()
        .skip(start)
        .take(view_h.max(1))
        .map(|l| Line::from(*l))
        .collect();
    f.render_widget(
        Paragraph::new(visible).style(style::text_style()),
        layout.content_rect,
    );
    scrollable_content::draw_scrollbar(f, &layout, total_lines);
}

fn yn_cell(is_yes_cell: bool, value_yes: bool, dimmed: bool) -> Span<'static> {
    let chosen = if is_yes_cell { value_yes } else { !value_yes };
    let label = if is_yes_cell { " Yes " } else { " No " };
    let st = if dimmed {
        if chosen {
            style::hint_text().add_modifier(Modifier::BOLD)
        } else {
            style::hint_text()
        }
    } else if chosen {
        style::tab_active()
    } else {
        style::tab_inactive()
    };
    Span::styled(label.to_string(), st)
}

/// Draw the Settings tab into `area` (typically full `main_area`).
pub fn draw_settings_pane(f: &mut Frame, area: Rect, state: &mut UblxState, dir_to_ublx: &Path) {
    let paths = UblxPaths::new(dir_to_ublx);
    let global_label = UI_STRINGS.config.global;
    let local_label = UI_STRINGS.config.local;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_style(style::panel_focused())
        .title_style(style::panel_title_style(true));
    let right_block = Block::default()
        .borders(Borders::ALL)
        .title(" File ")
        .border_style(style::panel_unfocused())
        .title_style(style::panel_title_style(false));

    let left_inner = left_block.inner(chunks[0]);
    let right_inner = right_block.inner(chunks[1]);
    f.render_widget(left_block, chunks[0]);
    f.render_widget(right_block, chunks[1]);

    let scope = state.settings.scope;
    let scope_spans = scope_tab_spans(scope, global_label, local_label);

    let global_path_str = paths.global_config().map_or_else(
        || "(global config path unavailable)".to_owned(),
        |p| p.display().to_string(),
    );
    let local_path_str = paths.toml_path().map_or_else(
        || "(no local ublx.toml / .ublx.toml)".to_owned(),
        |p| p.display().to_string(),
    );

    let overlay =
        settings::resolve_config_path(&paths, scope).and_then(|p| load_ublx_toml(Some(p), None));

    let local_ctx =
        matches!(scope, SettingsConfigScope::Local).then(|| settings::local_edit_context(&paths));

    let n_bool = settings::bool_row_count(scope);

    let layout_dimmed = local_ctx
        .as_ref()
        .is_some_and(|(loc, _)| !settings::local_layout_is_explicit(loc.as_ref()));

    let path_wrap = usize::from(left_inner.width).max(1);

    let mut left_lines: Vec<Line> = vec![Line::from(scope_spans), Line::from("")];
    push_scope_path_header(
        &mut left_lines,
        scope,
        &global_path_str,
        &local_path_str,
        path_wrap,
    );
    left_lines.push(Line::from(""));

    push_bool_rows(
        &mut left_lines,
        scope,
        n_bool,
        state.settings.left_cursor,
        local_ctx.as_ref(),
        overlay.as_ref(),
    );
    left_lines.push(Line::from(""));
    push_layout_edit_section(&mut left_lines, state, n_bool, layout_dimmed);
    push_external_apps_section(&mut left_lines);

    f.render_widget(
        Paragraph::new(left_lines).style(style::text_style()),
        left_inner,
    );

    render_settings_toml_preview(f, right_inner, state, &paths, scope);
}
