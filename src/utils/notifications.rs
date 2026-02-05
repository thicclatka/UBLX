//! Log capture and TUI display: bumper (toast) and optional dev log panel.
//!
//! - **User mode**: bumper only (last N messages, level-colored); info/warn/error.
//! - **Dev mode** (`UBLX_DEV=1`): full tui-logger panel (scrollable) + bumper; debug/trace included.

use log::Level;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

pub const DEFAULT_BUMPER_CAP: usize = 100;
const BUMPER_DISPLAY_LINES: usize = 3;

/// One log line for the bumper / history.
#[derive(Clone, Debug)]
pub struct BumperMessage {
    pub level: Level,
    pub text: String,
}

/// Thread-safe ring buffer of recent log messages for the bumper.
#[derive(Clone)]
pub struct BumperBuffer {
    inner: std::sync::Arc<Mutex<VecDeque<BumperMessage>>>,
    cap: usize,
}

impl BumperBuffer {
    pub fn new(cap: usize) -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(VecDeque::with_capacity(cap))),
            cap,
        }
    }

    pub fn push(&self, level: Level, text: String) {
        let mut g = self.inner.lock().expect("bumper lock");
        if g.len() >= self.cap {
            g.pop_front();
        }
        g.push_back(BumperMessage { level, text });
    }

    /// Last N messages (newest last). Returns up to `n` messages.
    pub fn last_n(&self, n: usize) -> Vec<BumperMessage> {
        let g = self.inner.lock().expect("bumper lock");
        let len = g.len();
        let start = len.saturating_sub(n);
        g.range(start..).cloned().collect()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().expect("bumper lock").is_empty()
    }
}

/// Dev mode: full log panel (tui-logger) + bumper. User mode: bumper only.
pub fn is_dev_mode() -> bool {
    std::env::var("UBLX_DEV")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

static BUMPER_FOR_LOG: OnceLock<BumperBuffer> = OnceLock::new();
static TUI_DRAIN: OnceLock<tui_logger::Drain> = OnceLock::new();

/// Initialize logging: bumper buffer + env_logger. In dev, also feed tui_logger via its Drain.
/// Call once at startup. Pass a clone of your BumperBuffer; keep the original for rendering.
/// Default filter: dev = Trace, user = Warn; overridable with RUST_LOG.
pub fn init_logging(bumper: BumperBuffer, dev: bool) {
    let _ = BUMPER_FOR_LOG.set(bumper);
    if dev {
        let _ = TUI_DRAIN.set(tui_logger::Drain::new());
    }

    let default_filter = if dev { "trace" } else { "warn" };
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_filter));

    builder
        .format(|_buf, record| {
            if let Some(b) = BUMPER_FOR_LOG.get() {
                b.push(record.level(), format!("{}", record.args()));
            }
            if let Some(d) = TUI_DRAIN.get() {
                d.log(record);
            }
            Ok(())
        })
        .init();
}

/// Call each frame in dev mode so the tui-logger widget receives new events.
pub fn move_log_events() {
    tui_logger::move_events();
}

fn level_style(level: Level) -> Style {
    let color = match level {
        Level::Error => Color::Red,
        Level::Warn => Color::Yellow,
        Level::Info => Color::Cyan,
        Level::Debug => Color::Magenta,
        Level::Trace => Color::Gray,
    };
    Style::default().fg(color)
}

fn level_short(level: Level) -> &'static str {
    match level {
        Level::Error => "E",
        Level::Warn => "W",
        Level::Info => "I",
        Level::Debug => "D",
        Level::Trace => "T",
    }
}

/// Draw the bumper (one or a few lines at the bottom). Uses last N messages.
#[allow(dead_code)]
pub fn render_bumper(f: &mut Frame, area: Rect, bumper: &BumperBuffer) {
    let messages = bumper.last_n(BUMPER_DISPLAY_LINES);
    if messages.is_empty() {
        return;
    }

    let line = Line::from(
        messages
            .iter()
            .map(|m| {
                Span::styled(
                    format!(" [{}] {} ", level_short(m.level), m.text),
                    level_style(m.level).add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>(),
    );
    let para = Paragraph::new(line).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

/// Draw a bordered bumper block (e.g. one line at bottom). Caller can reserve bottom rect.
pub fn render_bumper_block(f: &mut Frame, area: Rect, bumper: &BumperBuffer) {
    let messages = bumper.last_n(BUMPER_DISPLAY_LINES);
    if messages.is_empty() {
        return;
    }

    let line = Line::from(
        messages
            .iter()
            .map(|m| {
                Span::styled(
                    format!(" [{}] {} ", level_short(m.level), m.text),
                    level_style(m.level).add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>(),
    );
    let para = Paragraph::new(line)
        .block(Block::default().borders(Borders::TOP).title(" Log "))
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

/// State for the dev log panel (tui-logger smart widget).
#[derive(Default)]
pub struct DevLogState {
    pub widget_state: tui_logger::TuiWidgetState,
}

/// Render the full log panel (dev only). Call after move_log_events().
pub fn render_dev_log_panel(f: &mut Frame, area: Rect, state: &DevLogState) {
    let widget = tui_logger::TuiLoggerSmartWidget::default()
        .title_target(" Log (dev) ")
        .style(Style::default())
        .state(&state.widget_state);
    f.render_widget(widget, area);
}
