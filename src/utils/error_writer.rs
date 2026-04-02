use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use chrono::Local;
use log::error;

use crate::config::get_log_path;
use crate::integrations::ZahirResult;

/// If `result` is `Err(e)`, log with `log::error!` using `msg` as the format string (must contain one `{}` for `e`), append the same line to `ublx.log` when its path was set ([`crate::utils::set_index_dir_for_ublx_log`]), then exit with [`crate::utils::EXIT_ERROR`]. Otherwise return the `Ok` value.
///
/// # Errors
///
/// Returns [`std::io::Error`] when appending the fatal line to the log file fails.
pub fn fatal_error_handler<T, E: std::fmt::Display>(result: Result<T, E>, msg: &str) -> T {
    match result {
        Ok(v) => v,
        Err(e) => {
            let line = msg.replacen("{}", &e.to_string(), 1);
            error!("{line}");
            try_append_fatal_line(&line);
            exit_error();
        }
    }
}

/// Resolved path to `ublx.log` under the indexed root ([`get_log_path`]), for fatal/panic appends only.
static UBLX_LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

/// After [`validate_dir`](crate::utils::validate_dir): pass the indexed project root. Stores [`get_log_path`](crate::config::get_log_path) — the real `ublx.log` path, not the directory path.
pub fn set_index_dir_for_ublx_log(dir_to_ublx: &Path) {
    let _ = UBLX_LOG_FILE_PATH.set(get_log_path(dir_to_ublx));
}

/// Chain [`try_write_panic_to_log`] before the current panic hook (usually stderr). Call after [`set_index_dir_for_ublx_log`]. [`crate::handlers::core::run_tui_session`] installs another hook that restores the terminal; that hook wraps whatever was active here.
pub fn install_panic_hook_with_ublx_log(dir_to_ublx: &Path) {
    set_index_dir_for_ublx_log(dir_to_ublx);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        try_write_panic_to_log(info);
        previous(info);
    }));
}

/// Append a single line to the resolved `ublx.log` (best-effort; ignores I/O errors).
pub fn try_append_fatal_line(message_ref: &str) {
    let Some(log_path) = UBLX_LOG_FILE_PATH.get() else {
        return;
    };
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let _ = append_line_to_log_path(log_path, &format!("[{ts}] fatal: {message_ref}"));
}

/// Best-effort panic line for debugging when stderr is unusable (e.g. raw TUI). Chains with the previous hook in `main` / `run_tui_session`.
pub fn try_write_panic_to_log(info: &std::panic::PanicHookInfo) {
    let Some(log_path) = UBLX_LOG_FILE_PATH.get() else {
        return;
    };
    let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
        *s
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.as_str()
    } else {
        "(panic payload not a string)"
    };
    let loc = info.location().map_or_else(
        || "unknown location".to_string(),
        |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
    );
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let _ = append_line_to_log_path(log_path, &format!("[{ts}] panic at {loc}: {payload}"));
}

fn append_line_to_log_path(log_path: &Path, line: &str) -> std::io::Result<()> {
    let mut f = File::options().create(true).append(true).open(log_path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

/// User-facing `log::error!` text and related log-line prefixes for nefax + zahir during indexing.
pub struct NefaxZahirErrors;

impl NefaxZahirErrors {
    /// Basename of the per-project log file (matches [`get_log_path`]).
    pub const LOG_FILE_BASENAME: &'static str = "ublx.log";

    /// Header line appended to the log when listing zahirscan phase failures.
    pub const ZAHIRSCAN_FAILURES_HEADER: &'static str = "Zahirscan failures:";

    /// Prefix for a single nefaxer error line in the log (includes trailing space before the message).
    pub const NEFAXER_LOG_LINE_PREFIX: &'static str = "Nefaxer failure: ";

    #[must_use]
    pub fn nefax_failed(err: impl std::fmt::Display) -> String {
        format!("nefax failed: {err}")
    }

    #[must_use]
    pub fn zahir_sequential_failed(err: impl std::fmt::Display) -> String {
        format!("zahir (sequential) failed: {err}")
    }

    #[must_use]
    pub fn zahir_stream_failed(err: impl std::fmt::Display) -> String {
        format!("zahir (stream) failed: {err}")
    }

    #[must_use]
    pub fn zahir_failures_log_write_failed(err: impl std::fmt::Display) -> String {
        format!(
            "failed to write zahir failures to {}: {err}",
            Self::LOG_FILE_BASENAME
        )
    }

    pub const ZAHIR_THREAD_PANICKED: &'static str = "zahir thread panicked";
}

/// Exit code for error/failure. Used when validation fails, index fails, or a fatal error is logged. Shared so scripting and callers get consistent values.
pub const EXIT_ERROR: i32 = 1;

/// Exit code for invalid CLI usage (wrong flag combinations).
pub const EXIT_CLI_USAGE: i32 = 2;

/// Exits the process with [`EXIT_ERROR`]. Use after logging a fatal error instead of calling `std::process::exit` directly.
pub fn exit_error() -> ! {
    std::process::exit(EXIT_ERROR)
}

/// Exits the process with [`EXIT_CLI_USAGE`].
pub fn exit_cli_usage() -> ! {
    std::process::exit(EXIT_CLI_USAGE)
}

/// `--enhance-all` only applies with `--snapshot-only` or `--full-snapshot`; otherwise print and exit.
pub fn exit_if_enhance_all_without_headless(enhance_all: bool, snapshot_headless: bool) {
    if enhance_all && !snapshot_headless {
        eprintln!("ublx: --enhance-all requires --snapshot-only or --full-snapshot");
        exit_cli_usage();
    }
}

/// Extension trait so we can call `.iter_failures()` / `.failures()` on [`ZahirResult`]
pub trait ZahirResultExt {
    fn iter_failures(&self) -> impl Iterator<Item = (&String, &String)>;

    /// All phase1 and phase2 failures as a slice-friendly vec.
    fn failures(&self) -> Vec<(&String, &String)> {
        self.iter_failures().collect()
    }
}

impl ZahirResultExt for ZahirResult {
    fn iter_failures(&self) -> impl Iterator<Item = (&String, &String)> {
        self.phase1_failed
            .iter()
            .chain(self.phase2_failed.iter())
            .map(|(p, e)| (p, e))
    }
}

fn append_failures_zahirscan(
    f: &mut File,
    header: &str,
    failures: &[(&String, &String)],
) -> std::io::Result<()> {
    writeln!(f, "{header}")?;
    for (p, e) in failures {
        writeln!(f, "  {p}: {e}")?;
    }
    Ok(())
}

/// If `zahir_result.phase1_failed` or `phase2_failed` are non-empty, append them to `dir_to_ublx/ublx.log`.
///
/// # Errors
///
/// Returns [`std::io::Error`] when creating or writing the log file fails.
pub fn write_zahir_failures_to_log(
    dir_to_ublx: &Path,
    zahir_result: &ZahirResult,
) -> std::io::Result<()> {
    let header = NefaxZahirErrors::ZAHIRSCAN_FAILURES_HEADER;
    let failures = zahir_result.failures();
    if failures.is_empty() {
        return Ok(());
    }
    let mut f = create_log_file(dir_to_ublx)?;
    append_failures_zahirscan(&mut f, header, &failures)?;
    Ok(())
}

/// Same as [`write_zahir_failures_to_log`], but if the log write fails, emit [`error!`] instead of returning.
pub fn write_zahir_failures_to_log_error(dir_to_ublx: &Path, zahir_result: &ZahirResult) {
    if let Err(e) = write_zahir_failures_to_log(dir_to_ublx, zahir_result) {
        error!("{}", NefaxZahirErrors::zahir_failures_log_write_failed(&e));
    }
}

/// Append a nefaxer error to `dir_to_ublx/ublx.log`.
///
/// # Errors
///
/// Returns [`std::io::Error`] when creating or writing the log file fails.
pub fn write_nefax_error_to_log(
    dir_to_ublx: &Path,
    err: &impl std::fmt::Display,
) -> std::io::Result<()> {
    let mut f = create_log_file(dir_to_ublx)?;
    writeln!(f, "{}{err}", NefaxZahirErrors::NEFAXER_LOG_LINE_PREFIX)?;
    Ok(())
}

fn create_log_file(dir_to_ublx: &Path) -> std::io::Result<File> {
    let log_path = get_log_path(dir_to_ublx);
    File::options().create(true).append(true).open(log_path)
}
