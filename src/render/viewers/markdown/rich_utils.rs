use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::layout::themes;
use crate::ui::UI_GLYPHS;

type StyledLines = Vec<Line<'static>>;

const INLINE_CODE_BG_PCT: f32 = 0.20;

/// Extensions treated as “attachment” links for trailing glyph selection.
const ATTACH_EXT: &[&str] = &[
    ".pdf", ".zip", ".gz", ".tar", ".tgz", ".bz2", ".7z", ".rar", ".doc", ".docx", ".xls",
    ".xlsx", ".ppt", ".pptx", ".odt", ".ods", ".rtf",
];

pub fn link_trailing_glyph_for_dest(dest: &str) -> char {
    glyph_for_markdown_link_dest(dest)
}

fn glyph_for_markdown_link_dest(dest: &str) -> char {
    let path = dest
        .split(|c| ['?', '#'].contains(&c))
        .next()
        .unwrap_or(dest);
    let lower = path.to_ascii_lowercase();
    for ext in ATTACH_EXT {
        if lower.ends_with(ext) {
            return UI_GLYPHS.markdown_attachment;
        }
    }
    UI_GLYPHS.markdown_link
}

#[derive(Clone, Copy)]
pub enum RichKind {
    Paragraph,
    Heading { level: u8 },
    Item,
}

pub struct RichBuilder {
    kind: RichKind,
    completed_lines: Vec<Line<'static>>,
    current_spans: Vec<Span<'static>>,
    pub bold: u32,
    pub italic: u32,
    pub strike: u32,
    pub underline: u32,
    link_depth: u32,
    /// Glyph to append after the outermost link’s visible text (`end_link`).
    link_pending_glyph: Option<char>,
    image_depth: u32,
}

impl RichBuilder {
    pub fn new(kind: RichKind) -> Self {
        Self {
            kind,
            completed_lines: Vec::new(),
            current_spans: Vec::new(),
            bold: 0,
            italic: 0,
            strike: 0,
            underline: 0,
            link_depth: 0,
            link_pending_glyph: None,
            image_depth: 0,
        }
    }

    fn heading_style(level: u8) -> Style {
        let t = themes::current();
        let base = Style::default().fg(t.tab_active_fg);
        if level == 1 {
            base.add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED)
        } else {
            base.add_modifier(Modifier::ITALIC)
                .add_modifier(Modifier::BOLD)
        }
    }

    fn base_style(&self) -> Style {
        let t = themes::current();
        match self.kind {
            RichKind::Paragraph | RichKind::Item => Style::default().fg(t.text),
            RichKind::Heading { level } => Self::heading_style(level),
        }
    }

    fn current_text_style(&self) -> Style {
        let mut st = self.base_style();
        let mut m = Modifier::empty();
        if self.bold > 0 {
            m |= Modifier::BOLD;
        }
        if self.italic > 0 {
            m |= Modifier::ITALIC;
        }
        if self.strike > 0 {
            m |= Modifier::CROSSED_OUT;
        }
        if self.underline > 0 {
            m |= Modifier::UNDERLINED;
        }
        if self.link_depth > 0 {
            m |= Modifier::UNDERLINED;
        }
        if !m.is_empty() {
            st = st.add_modifier(m);
        }
        st
    }

    fn push_leading_image_glyph(&mut self, ch: char) {
        let st = self.current_text_style();
        self.current_spans.push(Span::styled(format!("{ch} "), st));
    }

    fn append_link_trailing_glyph(&mut self, ch: char) {
        // Called after `link_depth` is back to 0 so the glyph is not underlined.
        let st = self.current_text_style();
        let frag = Span::styled(format!(" {ch}"), st);
        if !self.current_spans.is_empty() {
            self.current_spans.push(frag);
        } else if let Some(last) = self.completed_lines.last_mut() {
            let spans: Vec<Span<'static>> = last.iter().cloned().collect();
            let mut new_line = Vec::with_capacity(spans.len() + 1);
            new_line.extend(spans);
            new_line.push(frag);
            *last = Line::from(new_line);
        } else {
            self.current_spans.push(frag);
        }
    }

    /// Inline `[text](url)` — stores glyph for after link text; text is underlined until `end_link`.
    pub fn begin_link(&mut self, dest_url: &str) {
        if self.link_depth == 0 {
            self.link_pending_glyph = Some(glyph_for_markdown_link_dest(dest_url));
        }
        self.link_depth += 1;
    }

    pub fn end_link(&mut self) {
        self.link_depth = self.link_depth.saturating_sub(1);
        if self.link_depth == 0
            && let Some(ch) = self.link_pending_glyph.take()
        {
            self.append_link_trailing_glyph(ch);
        }
    }

    /// Inline `![alt](url)` — leading image glyph before alt text.
    pub fn begin_image(&mut self) {
        if self.image_depth == 0 {
            self.push_leading_image_glyph(UI_GLYPHS.markdown_image);
        }
        self.image_depth += 1;
    }

    pub fn end_image(&mut self) {
        self.image_depth = self.image_depth.saturating_sub(1);
    }

    fn inline_code_style() -> Style {
        let t = themes::current();
        let bg = themes::lighten_rgb(t.background, INLINE_CODE_BG_PCT);
        Style::default().fg(t.text).bg(bg)
    }

    pub fn push_text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let st = self.current_text_style();
        self.current_spans.push(Span::styled(s.to_string(), st));
    }

    pub fn push_inline_code(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let st = Self::inline_code_style();
        self.current_spans.push(Span::styled(s.to_string(), st));
    }

    pub fn soft_break(&mut self) {
        self.completed_lines
            .push(Line::from(std::mem::take(&mut self.current_spans)));
    }

    pub fn finish(mut self) -> StyledLines {
        if !self.current_spans.is_empty() || self.completed_lines.is_empty() {
            self.completed_lines
                .push(Line::from(std::mem::take(&mut self.current_spans)));
        }
        self.completed_lines
    }
}

pub fn handle_break(
    in_table_cell: bool,
    current_cell: &mut String,
    buf: &mut String,
    rich: &mut Option<RichBuilder>,
) {
    if in_table_cell {
        current_cell.push(' ');
    } else if let Some(r) = rich.as_mut() {
        r.soft_break();
    } else {
        buf.push('\n');
    }
}
