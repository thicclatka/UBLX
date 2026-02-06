use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use log::{debug, error};
use ratatui::Terminal;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

use crate::config::UblxOpts;
use crate::engine::db_ops;
use crate::handlers::nefax_ops;
use crate::handlers::zahir_ops;
use crate::utils::{canonicalize_dir_to_ublx, error_writer, notifications};

/// Which panel has focus (Categories or Contents; Metadata is read-only).
#[derive(Clone, Copy, Default)]
enum PanelFocus {
    #[default]
    Categories,
    Contents,
}

/// Right pane content: Viewer (file content), or Templates / Metadata / Writing (zahir sections).
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum RightPaneMode {
    Viewer,
    #[default]
    Templates,
    Metadata,
    Writing,
}

/// Simple 3-panel TUI: categories (left), contents (middle), preview (right). Filled from snapshot when .ublx exists.
/// Keys: Up/Down move selection, Left/Right or Tab switch panel, q/Esc exit.
/// Category string for directories in the snapshot (matches [db_ops::UblxDbCategory]).
const CATEGORY_DIRECTORY: &str = "Directory";

/// Per-pane content from zahir JSON. Templates always present; metadata and writing only if keys exist.
struct SectionedPreview {
    templates: String,
    metadata: Option<String>,
    writing: Option<String>,
}

fn sectioned_preview_from_zahir(value: &serde_json::Value) -> SectionedPreview {
    let templates = value
        .get("templates")
        .and_then(|t| serde_json::to_string_pretty(t).ok())
        .filter(|s| !s.is_empty() && s != "null" && s != "[]")
        .unwrap_or_else(|| "no template found".to_string());

    let metadata = value.as_object().and_then(|obj| {
        let parts: Vec<String> = obj
            .iter()
            .filter(|(k, _)| k.ends_with("_metadata"))
            .filter_map(|(_, v)| serde_json::to_string_pretty(v).ok())
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    });

    let writing = value
        .get("writing_footprint")
        .and_then(|w| serde_json::to_string_pretty(w).ok());

    SectionedPreview {
        templates,
        metadata,
        writing,
    }
}

pub fn run_ublx_vanilla(db_path: &Path, dir_to_ublx: &Path) -> io::Result<()> {
    let categories = db_ops::load_snapshot_categories(db_path).unwrap_or_default();
    let all_rows = db_ops::load_snapshot_rows_for_tui(db_path, None).unwrap_or_default();

    let mut focus = PanelFocus::default();
    let mut category_state = ListState::default();
    let mut content_state = ListState::default();
    let mut preview_scroll: u16 = 0;
    let mut prev_preview_key: Option<(usize, Option<usize>)> = None;
    let mut search_query = String::new();
    let mut search_active = false;
    let mut cached_tree: Option<(String, String)> = None;
    let mut help_visible = false;
    let mut right_pane_mode = RightPaneMode::default();
    // "All" is always first; select it by default
    category_state.select(Some(0));
    content_state.select(Some(0));

    let highlight_style = Style::default().add_modifier(Modifier::REVERSED);

    enable_raw_mode()?;
    let mut out = io::stdout();
    crossterm::execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    loop {
        // Index 0 = "All", 1.. = categories[0], categories[1], ...
        let filtered_categories: Vec<String> = if search_query.trim().is_empty() {
            categories.clone()
        } else {
            let q = search_query.trim();
            categories
                .iter()
                .filter(|cat| {
                    all_rows
                        .iter()
                        .any(|(path, c, _)| c == *cat && (path.contains(q) || c.contains(q)))
                })
                .cloned()
                .collect()
        };
        let category_idx = category_state.selected().unwrap_or(0);
        let selected_category = if category_idx == 0 {
            None
        } else {
            filtered_categories
                .get(category_idx - 1)
                .map(String::as_str)
        };
        let contents_rows: Vec<_> = match selected_category {
            None => all_rows.clone(),
            Some(cat) => all_rows
                .iter()
                .filter(|(_, c, _)| c == cat)
                .cloned()
                .collect(),
        };
        let filtered_contents_rows: Vec<_> = if search_query.trim().is_empty() {
            contents_rows.clone()
        } else {
            let q = search_query.trim();
            contents_rows
                .iter()
                .filter(|(path, category, _)| path.contains(q) || category.contains(q))
                .cloned()
                .collect()
        };
        let category_list_len = 1 + filtered_categories.len();
        if category_list_len > 0 {
            let idx = category_idx.min(category_list_len.saturating_sub(1));
            category_state.select(Some(idx));
        }
        let content_len = filtered_contents_rows.len();
        if content_len > 0 {
            let sel = content_state
                .selected()
                .unwrap_or(0)
                .min(content_len.saturating_sub(1));
            content_state.select(Some(sel));
        } else {
            content_state.select(None);
        }

        let content_sel = content_state.selected();
        let preview_key = (category_idx, content_sel);
        if prev_preview_key.as_ref() != Some(&preview_key) {
            preview_scroll = 0;
            prev_preview_key = Some(preview_key);
        }

        let (templates_content, metadata_content, writing_content, viewer_content) = {
            let selected = content_state
                .selected()
                .and_then(|i| filtered_contents_rows.get(i));
            match selected {
                Some((path, category, zahir_json)) => {
                    if *category == CATEGORY_DIRECTORY {
                        let tree_str = {
                            let use_cache = cached_tree
                                .as_ref()
                                .is_some_and(|(cached_path, _)| cached_path == path);
                            if use_cache {
                                cached_tree.as_ref().unwrap().1.clone()
                            } else {
                                let full_path = dir_to_ublx.join(path);
                                match Command::new("tree").arg(&full_path).output() {
                                    Ok(out) if out.status.success() => {
                                        let text =
                                            String::from_utf8_lossy(&out.stdout).into_owned();
                                        cached_tree = Some((path.clone(), text.clone()));
                                        text
                                    }
                                    Ok(out) => {
                                        let stderr = String::from_utf8_lossy(&out.stderr);
                                        cached_tree = None;
                                        format!(
                                            "tree failed: {}",
                                            stderr.trim().lines().next().unwrap_or("unknown")
                                        )
                                    }
                                    Err(e) => {
                                        cached_tree = None;
                                        format!("tree not available: {}", e)
                                    }
                                }
                            }
                        };
                        ("no template found".to_string(), None, None, Some(tree_str))
                    } else if zahir_json.is_empty() {
                        cached_tree = None;
                        ("no template found".to_string(), None, None, None)
                    } else {
                        cached_tree = None;
                        match serde_json::from_str::<serde_json::Value>(zahir_json) {
                            Ok(v) => {
                                let s = sectioned_preview_from_zahir(&v);
                                (s.templates, s.metadata, s.writing, None)
                            }
                            _ => ("no template found".to_string(), None, None, None),
                        }
                    }
                }
                None => {
                    cached_tree = None;
                    ("(select an item)".to_string(), None, None, None)
                }
            }
        };

        terminal.draw(|f| {
            let vertical = if search_active {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(f.area())
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1)])
                    .split(f.area())
            };
            let main_area = vertical[0];
            let search_area = if search_active {
                Some(vertical[1])
            } else {
                None
            };

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(50),
                    Constraint::Percentage(30),
                ])
                .split(main_area);

            let left_title = match focus {
                PanelFocus::Categories => " ► Categories ",
                _ => " Categories ",
            };
            let middle_title = match focus {
                PanelFocus::Contents => " ► Contents ",
                _ => " Contents ",
            };
            let left_block = Block::default()
                .borders(Borders::ALL)
                .border_style(if matches!(focus, PanelFocus::Categories) {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                })
                .title(left_title);
            let middle_block = Block::default()
                .borders(Borders::ALL)
                .border_style(if matches!(focus, PanelFocus::Contents) {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                })
                .title(middle_title);
            let right_title = match right_pane_mode {
                RightPaneMode::Viewer => " Viewer ",
                RightPaneMode::Templates => " Templates ",
                RightPaneMode::Metadata => " Metadata ",
                RightPaneMode::Writing => " Writing ",
            };
            let right_block = Block::default().borders(Borders::ALL).title(right_title);

            let category_items: Vec<ListItem> = {
                let mut items = vec![ListItem::new("All")];
                items.extend(
                    filtered_categories
                        .iter()
                        .map(|s| ListItem::new(s.as_str())),
                );
                items
            };
            // Fixed-width symbol so switching focus doesn't shift list content (▌ when focused, spaces when not)
            let cat_symbol = if matches!(focus, PanelFocus::Categories) {
                "▌ "
            } else {
                "  "
            };
            let category_list = List::new(category_items.clone())
                .block(left_block)
                .highlight_style(highlight_style)
                .highlight_symbol(cat_symbol)
                .highlight_spacing(HighlightSpacing::Always);
            f.render_stateful_widget(category_list, chunks[0], &mut category_state);

            let content_items: Vec<ListItem> = if filtered_contents_rows.is_empty() {
                vec![ListItem::new(if search_query.is_empty() {
                    "(no contents)"
                } else {
                    "(no matches)"
                })]
            } else {
                filtered_contents_rows
                    .iter()
                    .map(|(path, _, _)| ListItem::new(path.as_str()))
                    .collect()
            };
            let cont_symbol = if matches!(focus, PanelFocus::Contents) {
                "▌ "
            } else {
                "  "
            };
            let content_list = List::new(content_items.clone())
                .block(middle_block)
                .highlight_style(highlight_style)
                .highlight_symbol(cont_symbol)
                .highlight_spacing(HighlightSpacing::Always);
            f.render_stateful_widget(content_list, chunks[1], &mut content_state);

            let right_content = match right_pane_mode {
                RightPaneMode::Templates => templates_content.as_str(),
                RightPaneMode::Metadata => metadata_content
                    .as_deref()
                    .unwrap_or("(not available for this item)"),
                RightPaneMode::Writing => writing_content
                    .as_deref()
                    .unwrap_or("(not available for this item)"),
                RightPaneMode::Viewer => viewer_content
                    .as_deref()
                    .unwrap_or("(viewer — file content will load here)"),
            };

            let tab_active = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let tab_available = Style::default();
            let tabs: Vec<(RightPaneMode, &str)> = [
                (RightPaneMode::Templates, "Templates"),
                (RightPaneMode::Viewer, "Viewer"),
                (RightPaneMode::Metadata, "Metadata"),
                (RightPaneMode::Writing, "Writing"),
            ]
            .into_iter()
            .filter(|(mode, _)| match mode {
                RightPaneMode::Templates | RightPaneMode::Viewer => true,
                RightPaneMode::Metadata => metadata_content.is_some(),
                RightPaneMode::Writing => writing_content.is_some(),
            })
            .collect();
            let tab_spans: Vec<Span> = tabs
                .iter()
                .enumerate()
                .flat_map(|(i, (mode, label))| {
                    let style = if *mode == right_pane_mode {
                        tab_active
                    } else {
                        tab_available
                    };
                    let sep = if i < tabs.len() - 1 { " | " } else { "" };
                    vec![Span::styled(*label, style), Span::raw(sep)]
                })
                .collect();
            let tab_line = Line::from(tab_spans);

            f.render_widget(&right_block, chunks[2]);
            let right_inner = right_block.inner(chunks[2]);
            let right_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(right_inner);
            f.render_widget(Paragraph::new(tab_line), right_split[0]);
            f.render_widget(
                Paragraph::new(Text::from(right_content)).scroll((preview_scroll, 0)),
                right_split[1],
            );

            if let Some(area) = search_area {
                let search_line = format!(" / {search_query}");
                let search_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Search ");
                let search_para =
                    Paragraph::new(Text::from(search_line.as_str())).block(search_block);
                f.render_widget(search_para, area);
            }

            if help_visible {
                const HELP_STR: &str = r#"/          search (strict substring)
q / Esc    quit
h / l      focus Categories / Contents
j / k      move down / up in list
Shift+↑↓   scroll right pane (or Shift+J / Shift+K)
Tab        switch focus
t / v / m / w  right pane: Templates / Viewer / Metadata / Writing (m,w only if data exists)
Shift+V        cycle right pane tab (only tabs with data)
?          show this help"#;
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
        })?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(e) = event::read()?
        {
            if e.kind != KeyEventKind::Press {
                continue;
            }
            if help_visible {
                help_visible = false;
                continue;
            }
            if search_active {
                match e.code {
                    KeyCode::Esc => {
                        search_query.clear();
                        search_active = false;
                    }
                    KeyCode::Enter => {
                        search_active = false;
                    }
                    KeyCode::Backspace => {
                        search_query.pop();
                    }
                    KeyCode::Char(c) => {
                        search_query.push(c);
                    }
                    _ => {
                        // Let arrows, j/k, h/l, Tab etc. fall through to navigation
                    }
                }
                if matches!(
                    e.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace | KeyCode::Char(_)
                ) {
                    continue;
                }
            }
            let shift = e.modifiers.contains(KeyModifiers::SHIFT);
            match e.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('?') => {
                    help_visible = true;
                }
                KeyCode::Char('/') => {
                    search_active = true;
                }
                KeyCode::Char('V') => {
                    let available: Vec<RightPaneMode> = [
                        RightPaneMode::Templates,
                        RightPaneMode::Viewer,
                        RightPaneMode::Metadata,
                        RightPaneMode::Writing,
                    ]
                    .into_iter()
                    .filter(|m| match m {
                        RightPaneMode::Templates | RightPaneMode::Viewer => true,
                        RightPaneMode::Metadata => metadata_content.is_some(),
                        RightPaneMode::Writing => writing_content.is_some(),
                    })
                    .collect();
                    if !available.is_empty() {
                        let idx = available
                            .iter()
                            .position(|m| *m == right_pane_mode)
                            .unwrap_or(0);
                        let next = (idx + 1) % available.len();
                        right_pane_mode = available[next];
                    }
                }
                KeyCode::Char('v') => {
                    right_pane_mode = RightPaneMode::Viewer;
                }
                KeyCode::Char('t') => {
                    right_pane_mode = RightPaneMode::Templates;
                }
                KeyCode::Char('m') => {
                    if metadata_content.is_some() {
                        right_pane_mode = RightPaneMode::Metadata;
                    }
                }
                KeyCode::Char('w') => {
                    if writing_content.is_some() {
                        right_pane_mode = RightPaneMode::Writing;
                    }
                }
                KeyCode::Up if shift => {
                    preview_scroll = preview_scroll.saturating_sub(1);
                }
                KeyCode::Char('K') => {
                    preview_scroll = preview_scroll.saturating_sub(1);
                }
                KeyCode::Down if shift => {
                    preview_scroll = preview_scroll.saturating_add(1);
                }
                KeyCode::Char('J') => {
                    preview_scroll = preview_scroll.saturating_add(1);
                }
                KeyCode::Up | KeyCode::Char('k') => match focus {
                    PanelFocus::Categories => {
                        let n = 1 + filtered_categories.len();
                        if n > 0 {
                            let i = category_state.selected().unwrap_or(0);
                            category_state.select(Some(i.saturating_sub(1).min(n - 1)));
                        }
                    }
                    PanelFocus::Contents => {
                        let n = filtered_contents_rows.len();
                        if n > 0 {
                            let i = content_state.selected().unwrap_or(0);
                            content_state.select(Some(i.saturating_sub(1).min(n - 1)));
                        }
                    }
                },
                KeyCode::Down | KeyCode::Char('j') => match focus {
                    PanelFocus::Categories => {
                        let n = 1 + filtered_categories.len();
                        if n > 0 {
                            let i = category_state.selected().unwrap_or(0);
                            category_state.select(Some((i + 1).min(n - 1)));
                        }
                    }
                    PanelFocus::Contents => {
                        let n = filtered_contents_rows.len();
                        if n > 0 {
                            let i = content_state.selected().unwrap_or(0);
                            content_state.select(Some((i + 1).min(n - 1)));
                        }
                    }
                },
                KeyCode::Left | KeyCode::Char('h') => focus = PanelFocus::Categories,
                KeyCode::Right | KeyCode::Char('l') => focus = PanelFocus::Contents,
                KeyCode::Tab => {
                    focus = match focus {
                        PanelFocus::Categories => PanelFocus::Contents,
                        PanelFocus::Contents => PanelFocus::Categories,
                    };
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[allow(dead_code)]
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
    // Zahir opens paths with File::open(path) so paths must be absolute (cwd-independent).
    let dir_to_ublx_abs = dir_to_ublx
        .canonicalize()
        .unwrap_or_else(|_| dir_to_ublx.to_path_buf());
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
                .map(|(p, _)| dir_to_ublx_abs.join(p))
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
    // Zahir opens paths with File::open(path) so paths must be absolute (cwd-independent).
    let dir_to_ublx_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let ublx_opts_for_zahir = ublx_opts.clone();
    let (tx, rx) = mpsc::channel();
    let zahir_handle =
        std::thread::spawn(move || zahir_ops::run_zahir_from_stream(rx, &ublx_opts_for_zahir));
    let on_entry = |e: &nefax_ops::NefaxEntry| {
        if e.size > 0 {
            let abs = dir_to_ublx_abs.join(&e.path).to_string_lossy().into_owned();
            let _ = tx.send(abs);
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
                &dir_to_ublx_abs,
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
