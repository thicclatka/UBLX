//! Video raster preview: **ffmpeg** decodes one frame; duration for midpoint seek uses
//! [`crate::integrations::run_ffprobe_safe`] (zahirscan’s safe ffprobe path).
//! Picks a **mid-timeline** frame when duration is known; otherwise ~1s in.

use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use image::DynamicImage;

use crate::integrations::run_ffprobe_safe;

/// Seconds to seek when duration is unknown (ffprobe missing or probe failed): skips typical leader black.
const FALLBACK_SEEK_SECS: &str = "1";

fn duration_secs_from_probe(path: &Path) -> Option<f64> {
    let probe = run_ffprobe_safe(path).ok()?;
    probe
        .format
        .duration
        .as_ref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.0)
}

/// Decode one preview frame to a [`DynamicImage`] (PNG over the pipe).
///
/// Seeks to roughly the **middle** of the clip when duration is known; otherwise about one second in.
/// Uses input seek (`-ss` before `-i`) for speed.
///
/// # Errors
///
/// Returns a short message when `ffmpeg` cannot be run, exits non-zero, or output is not a valid image.
pub fn decode_preview_frame(path: &Path) -> Result<DynamicImage, String> {
    let path_str = path.to_str().ok_or("invalid path")?;
    let seek = duration_secs_from_probe(path)
        .map(|d| format!("{}", d * 0.5))
        .unwrap_or_else(|| FALLBACK_SEEK_SECS.to_string());

    let out = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            seek.as_str(),
            "-i",
            path_str,
            "-vframes",
            "1",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "-",
        ])
        .output()
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                "ffmpeg not found. Install FFmpeg for video preview (https://ffmpeg.org/download.html)."
                    .to_string()
            } else {
                format!("ffmpeg ({e})")
            }
        })?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    image::load_from_memory(&out.stdout).map_err(|e| e.to_string())
}
