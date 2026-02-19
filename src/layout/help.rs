use ratatui::Frame;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::{style, themes};

pub const HELP_STR: &str = r#"1 / 2      main tabs: Snapshot / Delta
Shift+Tab  alternate Snapshot ↔ Delta
/          search (strict substring)
Enter      hide search bar (filter stays); Esc to clear search
Shift+S    take snapshot (runs in background; bumper when done)
q / Esc    quit
h / l      focus Categories / Contents
j / k      move down / up in list
gg / G     go to top / bottom of list (Categories or Contents)
Ctrl+b / Ctrl+e  viewer: scroll to beginning / end of preview
Shift+↑↓   scroll right pane; or double-tap Shift+J / Shift+K to scroll down / up
Tab        switch focus
t / v / m / w  right pane: Templates / Viewer / Metadata / Writing (m,w only if data exists)
Shift+V        cycle right pane tab (only tabs with data)
Shift+T       theme selector (j/k preview, Enter save to .ublx.toml, Esc revert)
?          show this help"#;

pub fn render_help_box(f: &mut Frame) {
    let area = f.area();
    let content_w = HELP_STR.lines().map(str::len).max().unwrap_or(0);
    let content_h = HELP_STR.lines().count();
    let rect = style::centered_popup_rect(area, content_w, content_h, 2, 2);
    f.render_widget(Clear, rect);

    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .border_style(Style::default().fg(t.focused_border))
        .style(Style::default().bg(t.popup_bg));
    let inner = Style::default().fg(t.text).bg(t.popup_bg);
    let para = Paragraph::new(Text::from(HELP_STR))
        .block(block)
        .style(inner);

    f.render_widget(para, rect);
}
