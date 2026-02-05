use log::{debug, error};
use std::path::PathBuf;

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

pub fn build_logger_test_mode_no_tui() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    debug!("test mode logger enabled");
}
