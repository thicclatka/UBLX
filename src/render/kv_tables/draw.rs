//! Viewport math and drawing tables to the frame.

use ratatui::layout::Rect;

use super::consts::TABLE_GAP;
use super::sections;
use super::sections::Section;
use crate::layout::style;
use crate::render::tables;
use crate::ui::UI_STRINGS;

/// Visible line range for a section: (skip_lines, take_lines) or None if section is off-screen.
fn visible_section_window(
    section_start: u16,
    section_height: u16,
    visible_start: u16,
    visible_end: u16,
) -> Option<(u16, u16)> {
    if section_start + section_height <= visible_start || section_start >= visible_end {
        return None;
    }
    let skip_lines = visible_start.saturating_sub(section_start);
    let take_lines =
        (section_height - skip_lines).min(visible_end.saturating_sub(section_start) - skip_lines);
    Some((skip_lines, take_lines))
}

/// Draw a section title line when it falls in the visible window. If `sub_title` is true, use subordinate style (e.g. for "TableName · Columns").
fn draw_section_title(
    f: &mut ratatui::Frame,
    title: &str,
    table_area: Rect,
    visible_start: u16,
    visible_end: u16,
    section_start: u16,
    sub_title: bool,
) {
    if section_start >= visible_start
        && section_start < visible_end
        && table_area.y + section_start.saturating_sub(visible_start)
            < table_area.y + table_area.height
    {
        let title_style = if sub_title {
            style::table_section_subtitle_style()
        } else {
            style::table_section_title_style()
        };
        let line = ratatui::text::Line::from(title).style(title_style);
        let ry = table_area.y + section_start.saturating_sub(visible_start);
        f.render_widget(
            ratatui::widgets::Paragraph::new(line),
            Rect {
                x: table_area.x,
                y: ry,
                width: table_area.width,
                height: 1,
            },
        );
    }
}

/// Rect for content at `y_offset` (from table_area.y) with height clamped so it doesn't exceed the viewport.
fn rect_in_viewport(table_area: Rect, y_offset: u16, height: u16, viewport: u16) -> Rect {
    let max_h = viewport.saturating_sub(y_offset);
    Rect {
        x: table_area.x,
        y: table_area.y + y_offset,
        width: table_area.width,
        height: height.min(max_h),
    }
}

/// Render Key/Value and Contents tables from JSON in the given rect, with scroll offset. Only the visible window is drawn; Contents rows are built only for visible range (memory optimization).
pub fn draw_tables(f: &mut ratatui::Frame, area: Rect, json: &str, scroll_y: u16) {
    use ratatui::widgets::Paragraph;

    let sections = sections::parse_json_sections(json);
    if sections.is_empty() {
        f.render_widget(
            Paragraph::new(UI_STRINGS.not_available).style(style::text_style()),
            area,
        );
        return;
    }
    let table_area = style::rect_with_h_pad(area);
    let viewport = table_area.height;
    let visible_start = scroll_y;
    let visible_end = scroll_y + viewport;
    let mut line_index: u16 = 0;
    let mut row_offset = 0;
    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            line_index += TABLE_GAP;
        }
        let (title_opt, header_lines, num_rows): (Option<&str>, u16, usize) = match section {
            Section::KeyValue(kv) => (kv.title.as_deref(), 1, kv.rows.len()),
            Section::Contents(c) => (Some(c.title.as_str()), 1, c.entries.len()),
            Section::SingleColumnList(list) => (Some(list.title.as_str()), 0, list.values.len()),
        };
        let section_start = line_index;
        if title_opt.is_some() {
            line_index += 1;
        }
        line_index += header_lines;
        line_index += num_rows as u16;
        let section_height = line_index - section_start;

        let Some((_skip_lines, take_lines)) =
            visible_section_window(section_start, section_height, visible_start, visible_end)
        else {
            row_offset += num_rows;
            continue;
        };

        match section {
            Section::KeyValue(kv) => {
                if let Some(title) = title_opt {
                    draw_section_title(
                        f,
                        title,
                        table_area,
                        visible_start,
                        visible_end,
                        section_start,
                        kv.sub_title,
                    );
                }
                let table_start_line = section_start + if title_opt.is_some() { 1 } else { 0 };
                let skip =
                    (visible_start.saturating_sub(table_start_line)).min(num_rows as u16) as usize;
                let take = (take_lines as usize).min(num_rows.saturating_sub(skip));
                if take > 0 {
                    let kv_visible = sections::KvSection {
                        title: None,
                        rows: kv.rows[skip..skip + take].to_vec(),
                        sub_title: false,
                    };
                    let actual_y = table_start_line.saturating_sub(visible_start);
                    let rect = rect_in_viewport(table_area, actual_y, (1 + take) as u16, viewport);
                    f.render_widget(
                        tables::section_to_table(&kv_visible, row_offset + skip),
                        rect,
                    );
                }
            }
            Section::Contents(c) => {
                if title_opt.is_some() {
                    draw_section_title(
                        f,
                        c.title.as_str(),
                        table_area,
                        visible_start,
                        visible_end,
                        section_start,
                        c.sub_title,
                    );
                }
                let table_start = section_start + 1;
                let skip =
                    (visible_start.saturating_sub(table_start)).min(num_rows as u16) as usize;
                let take = (take_lines as usize)
                    .min(num_rows.saturating_sub(skip))
                    .min((viewport as usize).saturating_sub(1));
                if take > 0 {
                    let y_offset = table_start.saturating_sub(visible_start);
                    let rect = rect_in_viewport(table_area, y_offset, (1 + take) as u16, viewport);
                    f.render_widget(
                        tables::contents_to_table_window(
                            c,
                            row_offset + skip,
                            skip,
                            skip + take,
                            rect.width,
                        ),
                        rect,
                    );
                }
            }
            Section::SingleColumnList(list) => {
                if title_opt.is_some() {
                    draw_section_title(
                        f,
                        list.title.as_str(),
                        table_area,
                        visible_start,
                        visible_end,
                        section_start,
                        false,
                    );
                }
                let table_start = section_start + 1;
                let skip =
                    (visible_start.saturating_sub(table_start)).min(num_rows as u16) as usize;
                let take = (take_lines as usize).min(num_rows.saturating_sub(skip));
                if take > 0 {
                    let y_offset = table_start.saturating_sub(visible_start);
                    let rect = rect_in_viewport(table_area, y_offset, take as u16, viewport);
                    f.render_widget(
                        tables::single_column_list_to_table(list, row_offset, skip, skip + take),
                        rect,
                    );
                }
            }
        }
        row_offset += num_rows;
    }
}
