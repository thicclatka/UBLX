use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use log::{debug, error};
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::CrosstermBackend;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;

use crate::config::UblxOpts;
use crate::engine::db_ops;
use crate::handlers::nefax_ops;
use crate::handlers::zahir_ops;
use crate::utils::{error_writer, notifications};

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

pub fn run_sequential(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
) -> io::Result<()> {
    let entry_callback: Option<fn(&nefax_ops::NefaxEntry)> = None;
    match nefax_ops::run_nefaxer(dir_to_ublx, ublx_opts, prior_nefax.as_ref(), entry_callback) {
        Ok((nefax, diff)) => {
            debug!(
                "indexed {} paths (added: {}, removed: {}, modified: {})",
                nefax.len(),
                diff.added.len(),
                diff.removed.len(),
                diff.modified.len()
            );
            let path_list: Vec<PathBuf> = nefax
                .iter()
                .filter(|(_, meta)| meta.size > 0)
                .map(|(p, _)| p.clone())
                .collect();
            let zahir_result = match zahir_ops::run_zahir_batch(&path_list, ublx_opts) {
                Ok(r) => r,
                Err(e) => {
                    error!("zahir (sequential) failed: {}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = error_writer::write_zahir_failures_to_log(dir_to_ublx, &zahir_result) {
                error!("failed to write zahir failures to ublx.log: {}", e);
            }
            if let Err(e) = db_ops::write_snapshot_to_db(
                dir_to_ublx,
                &nefax,
                &zahir_result,
                &diff,
                &ublx_opts.to_ublx_settings(),
            ) {
                error!("failed to write snapshot: {}", e);
                std::process::exit(1);
            }
            Ok(())
        }
        Err(e) => {
            let _ = error_writer::write_nefax_error_to_log(dir_to_ublx, &e);
            error!("nefax failed: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_stream(
    dir_to_ublx: &Path,
    ublx_opts: &UblxOpts,
    prior_nefax: &Option<nefax_ops::NefaxResult>,
) -> io::Result<()> {
    let ublx_opts_for_zahir = ublx_opts.clone();
    let (tx, rx) = mpsc::channel();
    let zahir_handle =
        std::thread::spawn(move || zahir_ops::run_zahir_from_stream(rx, &ublx_opts_for_zahir));
    let on_entry = |e: &nefax_ops::NefaxEntry| {
        if e.size > 0 {
            let _ = tx.send(e.path.to_string_lossy().into_owned());
        }
    };
    match nefax_ops::run_nefaxer(dir_to_ublx, ublx_opts, prior_nefax.as_ref(), Some(on_entry)) {
        Ok((nefax, diff)) => {
            drop(tx);
            debug!("indexed {} paths (streaming)", nefax.len());
            let zahir_result = match zahir_handle.join() {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    error!("zahir (stream) failed: {}", e);
                    std::process::exit(1);
                }
                Err(_) => {
                    error!("zahir thread panicked");
                    std::process::exit(1);
                }
            };
            if let Err(e) = error_writer::write_zahir_failures_to_log(dir_to_ublx, &zahir_result) {
                error!("failed to write zahir failures to log: {}", e);
            }
            if let Err(e) = db_ops::write_snapshot_to_db(
                dir_to_ublx,
                &nefax,
                &zahir_result,
                &diff,
                &ublx_opts.to_ublx_settings(),
            ) {
                error!("failed to write snapshot: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            drop(tx);
            let _ = zahir_handle.join();
            let _ = error_writer::write_nefax_error_to_log(dir_to_ublx, &e);
            error!("nefax failed: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}
