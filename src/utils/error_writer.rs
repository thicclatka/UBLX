use std::fs::File;
use std::io::Write;
use std::path::Path;

use log::error;

use crate::config::get_log_path;
use crate::integrations::ZahirResult;

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

/// Exits the process with [`EXIT_ERROR`]. Use after logging a fatal error instead of calling `std::process::exit` directly.
pub fn exit_error() -> ! {
    std::process::exit(EXIT_ERROR)
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
