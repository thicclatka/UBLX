//! Detect a CLI that reads UTF-8 from **stdin** and places it on the system clipboard.
//! Resolved once per session and cached on [`crate::layout::setup::UblxState::clipboard_copy`].

use std::io::Write;
use std::process::{Command, Stdio};

/// Spawn argv[0] with argv[1..], write bytes to stdin — used for `pbcopy`, `wl-copy`, `xclip`, `clip`, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardCopyCommand {
    pub argv: Vec<String>,
}

impl ClipboardCopyCommand {
    /// Probe PATH / standard tools once. Order: macOS `pbcopy`, Windows `clip`, then
    /// `wl-copy`, `xclip`, `xsel` on other Unix.
    #[must_use]
    pub fn detect() -> Option<Self> {
        #[cfg(target_os = "macos")]
        {
            unix_has_executable("pbcopy").then(|| Self {
                argv: vec!["pbcopy".to_owned()],
            })
        }
        #[cfg(windows)]
        {
            windows_has_clip().then(|| Self {
                argv: vec!["clip".to_owned()],
            })
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if unix_has_executable("wl-copy") {
                return Some(Self {
                    argv: vec!["wl-copy".to_owned()],
                });
            }
            if unix_has_executable("xclip") {
                return Some(Self {
                    argv: vec![
                        "xclip".to_owned(),
                        "-selection".to_owned(),
                        "clipboard".to_owned(),
                    ],
                });
            }
            if unix_has_executable("xsel") {
                return Some(Self {
                    argv: vec![
                        "xsel".to_owned(),
                        "--clipboard".to_owned(),
                        "--input".to_owned(),
                    ],
                });
            }
            return None;
        }
        #[cfg(not(any(unix, windows)))]
        {
            None
        }
    }

    /// Write `text` to the clipboard via the detected command (UTF-8 bytes on stdin).
    ///
    /// # Errors
    ///
    /// I/O errors from spawning or writing, or a non-zero exit from the helper.
    pub fn copy_utf8(&self, text: &str) -> std::io::Result<()> {
        let (prog, args) = self.argv.split_first().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "clipboard argv is empty")
        })?;
        let mut child = Command::new(prog)
            .args(args)
            .stdin(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("clipboard command has no stdin"))?;
        stdin.write_all(text.as_bytes())?;
        drop(stdin);
        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "clipboard command exited with {status}"
            )))
        }
    }
}

#[cfg(unix)]
fn unix_has_executable(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {}", name)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(windows)]
fn windows_has_clip() -> bool {
    Command::new("where")
        .arg("clip")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
