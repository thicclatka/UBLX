use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::CrosstermBackend;
use std::io;

use crate::utils::notifications;

pub fn run_ublx(bumper: &notifications::BumperBuffer, dev: bool) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let dev_log_state = notifications::DevLogState::default();

    loop {
        if dev {
            notifications::move_log_events();
        }
        terminal.draw(|f| {
            let chunks = if dev {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(10), Constraint::Length(3)])
                    .split(f.area())
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(f.area())
            };
            let main_area = chunks[0];
            let bumper_area = chunks[1];

            if dev {
                notifications::render_dev_log_panel(f, main_area, &dev_log_state);
            }
            notifications::render_bumper_block(f, bumper_area, bumper);
        })?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(e) = event::read()?
            && e.kind == KeyEventKind::Press
            && (e.code == KeyCode::Char('q') || e.code == KeyCode::Esc)
        {
            break;
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
