use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph};

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
?          show this help"#;

pub fn render_help_box(f: &mut Frame) {
    let area = f.area();
    let content_w = HELP_STR.lines().map(|l| l.len()).max().unwrap_or(0);
    let popup_w = (content_w + 2).min(area.width as usize) as u16; // +2 for block borders
    let popup_h = (HELP_STR.lines().count() + 2).min(area.height as usize) as u16; // +2 for block borders/title
    let x = area.x + area.width.saturating_sub(popup_w) / 2;
    let y = area.y + area.height.saturating_sub(popup_h) / 2;
    let popup_rect = Rect::new(x, y, popup_w, popup_h);
    let help_text = Text::from(HELP_STR);
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black))
        .title(" Help ");
    let help_para = Paragraph::new(help_text)
        .block(help_block)
        .style(Style::default().bg(Color::Black));
    f.render_widget(help_para, popup_rect);
}
