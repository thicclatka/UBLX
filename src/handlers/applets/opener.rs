//! Opener applet: open file in external editor (Terminal) or with OS default app (GUI).

use std::path::Path;
use std::process::Command;

/// Label for the viewer footer open hint: ↗ when both Terminal and GUI work, or "↗ (Terminal)" / "↗ (GUI)" when only one works. None when neither is available.
#[must_use]
pub fn open_hint_label(editor_path: Option<&str>) -> Option<&'static str> {
    let terminal = editor_path.is_some() || std::env::var("EDITOR").is_ok();
    let gui = gui_available();
    match (terminal, gui) {
        (true, true) => Some("↗"),
        (true, false) => Some("↗ (Terminal)"),
        (false, true) => Some("↗ (GUI)"),
        (false, false) => None,
    }
}

fn gui_available() -> bool {
    #[cfg(any(target_os = "macos", all(unix, not(target_os = "macos")), windows))]
    {
        true
    }
    #[cfg(not(any(target_os = "macos", all(unix, not(target_os = "macos")), windows)))]
    {
        false
    }
}

/// Resolve editor command: config `editor_path` or $EDITOR. Returns None if neither set.
pub fn editor_for_open(editor_path: Option<&str>) -> Option<String> {
    editor_path
        .map(ToString::to_string)
        .or_else(|| std::env::var("EDITOR").ok())
}

/// Spawn editor for the given path and wait for it to exit. Returns true if the process was started and exited (no guarantee the file was saved).
///
/// # Errors
///
/// Returns [`std::io::Error`] if spawning the editor or waiting for exit fails.
pub fn open_in_editor(editor: &str, path: &Path) -> std::io::Result<bool> {
    let status = Command::new(editor).arg(path).status()?;
    Ok(status.code().is_some())
}

/// Open path with the OS default application (e.g. open on macOS, xdg-open on Linux).
///
/// # Errors
///
/// Returns [`std::io::Error`] if spawning the helper process fails.
#[cfg(target_os = "macos")]
pub fn open_in_gui(path: &Path) -> std::io::Result<std::process::Child> {
    Command::new("open").arg(path).spawn()
}

/// # Errors
///
/// Returns [`std::io::Error`] if spawning the helper process fails.
#[cfg(all(unix, not(target_os = "macos")))]
pub fn open_in_gui(path: &Path) -> std::io::Result<std::process::Child> {
    Command::new("xdg-open").arg(path).spawn()
}

/// # Errors
///
/// Returns [`std::io::Error`] if spawning the helper process fails.
#[cfg(windows)]
pub fn open_in_gui(path: &Path) -> std::io::Result<std::process::Child> {
    Command::new("explorer").arg(path).spawn()
}

/// # Errors
///
/// Returns [`std::io::Error`] with kind [`std::io::ErrorKind::Unsupported`] on this platform.
#[cfg(not(any(unix, windows)))]
pub fn open_in_gui(_path: &Path) -> std::io::Result<std::process::Child> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Open (GUI) not supported on this platform",
    ))
}
