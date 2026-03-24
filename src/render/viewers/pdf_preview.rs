//! PDF page rasterization via **Poppler** [`pdftoppm`] / [`pdfinfo`] or **`MuPDF`** [`mutool`].
//! Install: `brew install poppler` / `brew install mupdf` (names vary by distro).

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::DynamicImage;

use crate::utils::unique_stamp;

/// Extra scale on the PDF raster longest edge vs plain images (applied after viewport + tier caps).
///
/// Tunable via [`PdfRasterMaxDimBoost::NUMERATOR`] / [`PdfRasterMaxDimBoost::DENOMINATOR`] (e.g. `6/4`) and [`PdfRasterMaxDimBoost::CAP_PX`].
pub struct PdfRasterMaxDimBoost;

impl PdfRasterMaxDimBoost {
    pub const NUMERATOR: u64 = 6;
    pub const DENOMINATOR: u64 = 4;
    /// Upper bound on boosted longest edge (px) so decode/encode stays bounded.
    pub const CAP_PX: u64 = 3000;

    /// Viewport-derived longest edge → boosted cap for PDF rasterization.
    #[inline]
    #[must_use]
    pub fn apply(base: u32) -> u32 {
        ((base as u64 * Self::NUMERATOR) / Self::DENOMINATOR).min(Self::CAP_PX) as u32
    }
}

/// One failed attempt: whether the executable was missing from `PATH` vs ran and failed.
struct ToolAttempt {
    missing_binary: bool,
    message: String,
}

/// Page count from Poppler [`pdfinfo`] (same package as `pdftoppm`).
pub fn pdf_page_count(pdf: &Path) -> Result<u32, String> {
    try_pdfinfo_pages(pdf)
}

fn try_pdfinfo_pages(pdf: &Path) -> Result<u32, String> {
    let out = Command::new("pdfinfo")
        .arg(pdf.as_os_str())
        .output()
        .map_err(|e| format!("pdfinfo ({e})"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Pages:") {
            return rest
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("pdfinfo: invalid Pages line: {line}"));
        }
    }
    Err("pdfinfo: no Pages line".to_string())
}

/// Rasterize one-based page index `page` (≥ 1) with longest edge at most `max_dim` pixels.
pub fn render_pdf_page(pdf: &Path, page: u32, max_dim: u32) -> Result<DynamicImage, String> {
    let page = page.max(1);
    let tmp = std::env::temp_dir();
    let stamp = unique_stamp();

    let err_poppler = match try_pdftoppm(pdf, page, max_dim, &tmp, stamp) {
        Ok(png) => return load_png_remove(png),
        Err(a) => a,
    };

    let err_mutool = match try_mutool_draw(pdf, page, max_dim, &tmp, stamp) {
        Ok(png) => return load_png_remove(png),
        Err(a) => a,
    };

    if err_poppler.missing_binary && err_mutool.missing_binary {
        return Err(
            "PDF preview: install Poppler (pdftoppm) or MuPDF (mutool) to enable PDF preview."
                .to_string(),
        );
    }

    Err(format!("{}; {}", err_poppler.message, err_mutool.message))
}

fn load_png_remove(path: PathBuf) -> Result<DynamicImage, String> {
    let img = image::open(&path).map_err(|e| format!("decode rendered PNG: {e}"))?;
    let _ = fs::remove_file(&path);
    Ok(img)
}

fn try_pdftoppm(
    pdf: &Path,
    page: u32,
    max_dim: u32,
    tmp: &Path,
    stamp: u64,
) -> Result<PathBuf, ToolAttempt> {
    let p = page.max(1).to_string();
    let out_base = tmp.join(format!("ublx_pdf_pp_{stamp}"));
    let out_arg = out_base.to_string_lossy().to_string();
    let out = Command::new("pdftoppm")
        .arg("-f")
        .arg(&p)
        .arg("-l")
        .arg(&p)
        .args(["-png", "-singlefile"])
        .arg("-scale-to")
        .arg(max_dim.to_string())
        .arg(pdf.as_os_str())
        .arg(&out_arg)
        .output()
        .map_err(|e| ToolAttempt {
            missing_binary: e.kind() == ErrorKind::NotFound,
            message: format!("pdftoppm: {e}"),
        })?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ToolAttempt {
            missing_binary: false,
            message: format!(
                "pdftoppm: {}",
                stderr.trim().lines().next().unwrap_or("failed")
            ),
        });
    }

    let png = out_base.with_extension("png");
    if png.is_file() {
        Ok(png)
    } else {
        Err(ToolAttempt {
            missing_binary: false,
            message: format!("pdftoppm: expected {}", png.display()),
        })
    }
}

fn try_mutool_draw(
    pdf: &Path,
    page: u32,
    max_dim: u32,
    tmp: &Path,
    stamp: u64,
) -> Result<PathBuf, ToolAttempt> {
    let p = page.max(1).to_string();
    let png = tmp.join(format!("ublx_pdf_mu_{stamp}.png"));
    let out = Command::new("mutool")
        .arg("draw")
        .arg("-o")
        .arg(&png)
        .args(["-F", "png"])
        .arg("-w")
        .arg(max_dim.to_string())
        .arg(pdf.as_os_str())
        .arg(&p)
        .output()
        .map_err(|e| ToolAttempt {
            missing_binary: e.kind() == ErrorKind::NotFound,
            message: format!("mutool: {e}"),
        })?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let _ = fs::remove_file(&png);
        return Err(ToolAttempt {
            missing_binary: false,
            message: format!(
                "mutool: {}",
                stderr.trim().lines().next().unwrap_or("failed")
            ),
        });
    }

    if png.is_file() {
        Ok(png)
    } else {
        Err(ToolAttempt {
            missing_binary: false,
            message: "mutool: missing PNG output".to_string(),
        })
    }
}
