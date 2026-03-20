use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::config::get_log_path;
use crate::handlers::zahir_ops::ZahirResult;

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
    let header = "Zahirscan failures:";
    let failures = zahir_result.failures();
    if failures.is_empty() {
        return Ok(());
    }
    let mut f = create_log_file(dir_to_ublx)?;
    append_failures_zahirscan(&mut f, header, &failures)?;
    Ok(())
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
    writeln!(f, "Nefaxer failure: {err}")?;
    Ok(())
}

fn create_log_file(dir_to_ublx: &Path) -> std::io::Result<File> {
    let log_path = get_log_path(dir_to_ublx);
    File::options().create(true).append(true).open(log_path)
}
