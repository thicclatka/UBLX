use colored::Colorize;
use log::{Level, debug, error};
use std::io::Write;
use std::path::PathBuf;

use crate::config::PKG_NAME;

/// Validate that a path is a directory and return the canonicalized path.
pub fn validate_dir(path: &std::path::Path) -> PathBuf {
    if path.exists() && !path.is_dir() {
        error!("'{}' is not a directory", path.display());
        std::process::exit(1);
    }
    if !path.exists() {
        error!("'{}' no such file or directory", path.display());
        std::process::exit(1);
    }
    path.canonicalize().unwrap_or_else(|e| {
        error!("cannot canonicalize '{}': {}", path.display(), e);
        std::process::exit(1);
    })
}

/// Color the level of the log message.
fn level_colored(level: Level) -> String {
    let s = format!("{}", level);
    match level {
        Level::Error => s.red().to_string(),
        Level::Warn => s.yellow().to_string(),
        Level::Info => s.green().to_string(),
        Level::Debug => s.cyan().to_string(),
        Level::Trace => s.dimmed().to_string(),
    }
}

/// Color the path of the log message.
fn path_colored(target: &str) -> String {
    if target.contains("zahirscan") {
        target.bright_green().to_string()
    } else if target.contains("nefaxer") {
        target.bright_cyan().to_string()
    } else {
        target.magenta().to_string()
    }
}

/// Build a logger for test mode without the TUI.
pub fn build_logger_test_mode_no_tui() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .format(|buf, record| {
            let ublx = PKG_NAME.to_uppercase()
                .magenta()
                .bold()
                .to_string();
            let level = level_colored(record.level());
            let path = path_colored(record.target());
            writeln!(buf, "[{} {} {}] {}", ublx, level, path, record.args())
        })
        .init();
    debug!("UBLX test mode logger enabled");
}
