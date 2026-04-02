//! Viewport math and drawing tables to the frame.

use ratatui::layout::Rect;

use crate::layout::{setup::UblxState, style};
use crate::modules::viewer_search;
use crate::ui::UI_STRINGS;

use super::consts::TABLE_GAP;
use super::ratatui_table;
use super::sections;
use super::sections::{ContentsSection, KvSection, Section, SingleColumnListSection};

/// Visible line range for a section: (`skip_lines`, `take_lines`) or None if section is off-screen.
#[must_use]
pub fn visible_section_window(
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

/// Draw a section title line when it falls in the visible window. If `sub_title` is true, use subordinate style (e.g. for "`TableName` · Columns").
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
        let line = ratatui::text::Line::from(title.to_uppercase()).style(title_style);
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

/// Rect for content at `y_offset` (from `table_area.y`) with height clamped so it doesn't exceed the viewport.
#[must_use]
pub fn rect_in_viewport(table_area: Rect, y_offset: u16, height: u16, viewport: u16) -> Rect {
    let max_h = viewport.saturating_sub(y_offset);
    Rect {
        x: table_area.x,
        y: table_area.y + y_offset,
        width: table_area.width,
        height: height.min(max_h),
    }
}

/// First data row index, row count, and Y offset for a ratatui table in the padded area.
///
/// `table_start_line` is the first line of the table widget (after any section title line).
/// `take_extra_cap` is used for Contents tables only: at most `viewport - 1` data rows so the
/// header row plus rows fit in the viewport.
struct TableRowWindow {
    skip: usize,
    take: usize,
    y_offset: u16,
}

impl TableRowWindow {
    /// Height in terminal rows: either data rows only, or one header row plus data rows (KV / Contents).
    #[must_use]
    fn rect_height(self, include_header_row: bool) -> u16 {
        if include_header_row {
            (1 + self.take) as u16
        } else {
            self.take as u16
        }
    }
}

/// Frame + padded table rect + vertical scroll window (shared by per-section draw helpers).
struct TableDrawCtx<'a, 'f> {
    f: &'a mut ratatui::Frame<'f>,
    table_area: Rect,
    viewport: u16,
    visible_start: u16,
    find_needle: Option<&'a str>,
    /// Haystack line starts when find has synced ranges (for `n`/`N` cell highlight alignment).
    line_starts: Option<&'a [usize]>,
    find_ranges: &'a [(usize, usize)],
    find_current: usize,
}

impl TableDrawCtx<'_, '_> {
    /// Map scroll window to visible data rows. Returns `None` when nothing to draw.
    fn window_table_rows(
        &self,
        table_start_line: u16,
        take_lines: u16,
        num_rows: usize,
        take_extra_cap: Option<usize>,
    ) -> Option<TableRowWindow> {
        let skip =
            (self.visible_start.saturating_sub(table_start_line)).min(num_rows as u16) as usize;
        let mut take = (take_lines as usize).min(num_rows.saturating_sub(skip));
        if let Some(cap) = take_extra_cap {
            take = take.min(cap);
        }
        if take == 0 {
            return None;
        }
        Some(TableRowWindow {
            skip,
            take,
            y_offset: table_start_line.saturating_sub(self.visible_start),
        })
    }

    fn draw_kv_visible(
        &mut self,
        section_start: u16,
        take_lines: u16,
        has_title: bool,
        num_rows: usize,
        row_offset: usize,
        kv: &KvSection,
    ) {
        let table_start_line = section_start + u16::from(has_title);
        let Some(w) = self.window_table_rows(table_start_line, take_lines, num_rows, None) else {
            return;
        };
        let skip = w.skip;
        let take = w.take;
        let kv_visible = KvSection {
            title: None,
            rows: kv.rows[skip..skip + take].to_vec(),
            sub_title: false,
        };
        let rect = rect_in_viewport(
            self.table_area,
            w.y_offset,
            w.rect_height(true),
            self.viewport,
        );
        let first_data_line_idx = (section_start as usize) + usize::from(has_title) + 1;
        let find_kv_data = self.line_starts.and_then(|ls| {
            if self.find_ranges.is_empty() {
                return None;
            }
            Some(ratatui_table::KvFindSync {
                line_starts: ls,
                ranges: self.find_ranges,
                current: self.find_current,
                first_data_line_idx,
                row_skip: skip,
            })
        });
        self.f.render_widget(
            ratatui_table::section_to_table(
                &kv_visible,
                row_offset + skip,
                self.find_needle,
                find_kv_data.as_ref(),
            ),
            rect,
        );
    }

    fn draw_contents_visible(
        &mut self,
        section_start: u16,
        take_lines: u16,
        num_rows: usize,
        row_offset: usize,
        c: &ContentsSection,
    ) {
        let table_start_line = section_start + 1;
        let take_cap = (self.viewport as usize).saturating_sub(1);
        let Some(w) =
            self.window_table_rows(table_start_line, take_lines, num_rows, Some(take_cap))
        else {
            return;
        };
        let skip = w.skip;
        let take = w.take;
        let rect = rect_in_viewport(
            self.table_area,
            w.y_offset,
            w.rect_height(true),
            self.viewport,
        );
        self.f.render_widget(
            ratatui_table::contents_to_table_window(
                c,
                row_offset + skip,
                skip,
                skip + take,
                rect.width,
                self.find_needle,
            ),
            rect,
        );
    }

    fn draw_single_column_list_visible(
        &mut self,
        section_start: u16,
        take_lines: u16,
        num_rows: usize,
        row_offset: usize,
        list: &SingleColumnListSection,
    ) {
        let table_start_line = section_start + 1;
        let Some(w) = self.window_table_rows(table_start_line, take_lines, num_rows, None) else {
            return;
        };
        let skip = w.skip;
        let take = w.take;
        let rect = rect_in_viewport(
            self.table_area,
            w.y_offset,
            w.rect_height(false),
            self.viewport,
        );
        self.f.render_widget(
            ratatui_table::single_column_list_to_table(
                list,
                row_offset,
                skip,
                skip + take,
                self.find_needle,
            ),
            rect,
        );
    }
}

/// Render Key/Value and Contents tables from JSON in the given rect, with scroll offset. Only the visible window is drawn; Contents rows are built only for visible range (memory optimization).
pub fn draw_tables(
    f: &mut ratatui::Frame,
    area: Rect,
    json: &str,
    scroll_y: u16,
    find_needle: Option<&str>,
    state: &UblxState,
) {
    use ratatui::widgets::Paragraph;

    let sections = sections::parse_json_sections(json);
    if sections.is_empty() {
        f.render_widget(
            Paragraph::new(UI_STRINGS.pane.not_available).style(style::text_style()),
            area,
        );
        return;
    }
    let table_area = style::rect_with_h_pad(area);
    let viewport = table_area.height;
    let visible_start = scroll_y;
    let visible_end = scroll_y + viewport;
    let line_starts_vec = if viewer_search::option_needle_nonempty(find_needle)
        && !state.viewer_find.ranges.is_empty()
    {
        let hay = sections::searchable_text_from_json(json);
        Some(sections::line_byte_starts(&hay))
    } else {
        None
    };
    let line_starts = line_starts_vec.as_deref();
    let mut ctx = TableDrawCtx {
        f,
        table_area,
        viewport,
        visible_start,
        find_needle,
        line_starts,
        find_ranges: &state.viewer_find.ranges,
        find_current: state.viewer_find.current,
    };
    let mut line_index: u16 = 0;
    let mut row_offset = 0;
    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            line_index += TABLE_GAP;
        }
        let title_opt = section.title_str();
        let (_has_title, header_lines, num_rows) = section.line_metrics();
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

        if let Some(title) = title_opt {
            draw_section_title(
                ctx.f,
                title,
                ctx.table_area,
                ctx.visible_start,
                visible_end,
                section_start,
                section.sub_title_style(),
            );
        }
        let has_title = title_opt.is_some();
        match section {
            Section::KeyValue(kv) => {
                ctx.draw_kv_visible(
                    section_start,
                    take_lines,
                    has_title,
                    num_rows,
                    row_offset,
                    kv,
                );
            }
            Section::Contents(c) => {
                ctx.draw_contents_visible(section_start, take_lines, num_rows, row_offset, c);
            }
            Section::SingleColumnList(list) => {
                ctx.draw_single_column_list_visible(
                    section_start,
                    take_lines,
                    num_rows,
                    row_offset,
                    list,
                );
            }
        }
        row_offset += num_rows;
    }
}
