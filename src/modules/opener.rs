//! Opener applet: open file in external editor (Terminal) or with OS default app (GUI).

use std::io;
use std::path::Path;
use std::process::Command;

/// Open `https://` / `http://` URL in the system browser.
///
/// # Errors
///
/// Returns [`std::io::Error`] if spawning the platform helper fails.
pub fn open_url(url: &str) -> io::Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()?;
    } else if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()?;
    } else if cfg!(unix) {
        Command::new("xdg-open").arg(url).spawn()?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "open URL not supported on this platform",
        ));
    }
    Ok(())
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

/// Open the system file manager focused on `path`: reveal in Finder (macOS), Explorer `/select` (Windows),
/// or open the parent directory in the default file manager (Linux and other Unix — selection varies by DE).
///
/// Branches use [`cfg!`] so each target only emits its own code; order matters because macOS is `unix`.
///
/// # Errors
///
/// Returns [`std::io::Error`] if spawning the helper fails, or [`std::io::ErrorKind::Unsupported`] on other targets.
pub fn reveal_in_file_manager(path: &Path) -> std::io::Result<std::process::Child> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg("-R").arg(path).spawn()
    } else if cfg!(windows) {
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let p = abs.to_string_lossy();
        Command::new("explorer").arg(format!("/select,{p}")).spawn()
    } else if cfg!(all(unix, not(target_os = "macos"))) {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(path);
        Command::new("xdg-open").arg(dir).spawn()
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Reveal in file manager not supported on this platform",
        ))
    }
}
