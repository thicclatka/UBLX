//! Log capture and TUI display: toast notifications from log messages.
//!
//! - **User mode**: toast (last N messages, level-colored); info/warn/error.
//! - **Dev mode** (`--dev`): tui-logger drain + move_log_events(); trace-level default filter.

use log::Level;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::config::TOAST_CONFIG;
use crate::layout::themes;
use crate::ui::UI_STRINGS;
use crate::utils::format::StringObjTraits;

static BUMPER_FOR_LOG: OnceLock<BumperBuffer> = OnceLock::new();
static TUI_DRAIN: OnceLock<tui_logger::Drain> = OnceLock::new();

/// One log line for the bumper / history.
#[derive(Clone, Debug)]
pub struct BumperMessage {
    pub level: Level,
    pub text: String,
    /// Optional operation name used as the toast title (e.g. "ublx-snapshot").
    pub operation: Option<String>,
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
        self.push_with_operation(level, text, None::<String>);
    }

    /// Push a message with an operation name; the toast title uses the most recent message's operation.
    pub fn push_with_operation(
        &self,
        level: Level,
        text: String,
        operation: Option<impl Into<String>>,
    ) {
        let mut g = self.inner.lock().expect("bumper lock");
        if g.len() >= self.cap {
            g.pop_front();
        }
        g.push_back(BumperMessage {
            level,
            text,
            operation: operation.map(Into::into),
        });
    }

    /// Last N messages (newest last). Returns up to `n` messages.
    pub fn last_n(&self, n: usize) -> Vec<BumperMessage> {
        let g = self.inner.lock().expect("bumper lock");
        let len = g.len();
        let start = len.saturating_sub(n);
        g.range(start..).cloned().collect()
    }
}

/// One stacked toast: snapshot of messages and its own timer.
#[derive(Clone, Debug)]
pub struct ToastSlot {
    pub visible_until: Instant,
    pub operation: Option<String>,
    pub messages: Vec<BumperMessage>,
}

/// Push a new toast onto the stack (snapshot from bumper, trim to max_toast_stack). Call after pushing to bumper.
pub fn show_toast_slot(
    slots: &mut Vec<ToastSlot>,
    bumper: &BumperBuffer,
    operation: Option<&str>,
    dev: bool,
) {
    let line_count = TOAST_CONFIG.display_lines_for_operation(dev, operation);
    let messages = bumper.last_n(line_count);
    if messages.is_empty() {
        return;
    }
    slots.push(ToastSlot {
        visible_until: Instant::now() + TOAST_CONFIG.duration,
        operation: operation.map(String::from),
        messages,
    });
    while slots.len() > TOAST_CONFIG.max_toast_stack {
        slots.remove(0);
    }
}

/// Write bumper contents to stderr. Call after the TUI exits so terminal is restored; safe to read in scrollback.
pub fn flush_bumper_to_stderr(bumper: &BumperBuffer) {
    let msgs = bumper.last_n(500);
    if msgs.is_empty() {
        return;
    }
    let mut out = std::io::stderr().lock();
    let _ = writeln!(out, "--- ublx log (last {} messages) ---", msgs.len());
    for m in &msgs {
        let prefix = level_short(m.level);
        let _ = writeln!(out, "{} {}", prefix, m.text);
    }
    let _ = out.flush();
}

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
    let colors = themes::DEFAULT_COLORS;
    let color = match level {
        Level::Error => colors.red,
        Level::Warn => colors.yellow,
        Level::Info => colors.cyan,
        Level::Debug => colors.magenta,
        Level::Trace => colors.gray,
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

/// Draw one toast slot in the given rect (used for stacked toasts).
pub fn render_toast_slot(f: &mut Frame, area: Rect, slot: &ToastSlot) {
    f.render_widget(Clear, area);
    if slot.messages.is_empty() {
        return;
    }
    let title = slot
        .messages
        .last()
        .and_then(|m| m.operation.as_deref())
        .map(|s| UI_STRINGS.pad(s))
        .unwrap_or_else(|| UI_STRINGS.pad(UI_STRINGS.notification_title));
    let lines: Vec<Line<'_>> = slot
        .messages
        .iter()
        .map(|m| {
            Line::from(Span::styled(
                format!(" [{}] {}", level_short(m.level), m.text),
                level_style(m.level).add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    let t = themes::current();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.focused_border).bg(t.notification_bg))
        .style(Style::default().bg(t.notification_bg))
        .title(title);
    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}
