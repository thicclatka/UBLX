//! Per-file `ZahirScan` enrichment when global `enable_enhance_all` is false.

use std::path::Path;

use crate::config::UblxOpts;
use crate::engine::db_ops;
use crate::integrations;
use crate::utils::canonicalize_dir_to_ublx;

/// Run `ZahirScan` on one file and update the snapshot row.
///
/// # Errors
///
/// Returns when zahirscan yields no output for the path or DB update fails.
pub fn enhance_single_path(
    dir_to_ublx: &Path,
    db_path: &Path,
    path_rel: &str,
    ublx_opts: &UblxOpts,
) -> Result<(), anyhow::Error> {
    let dir_abs = canonicalize_dir_to_ublx(dir_to_ublx);
    let abs = dir_abs.join(path_rel);
    let result = (|| {
        let zr = integrations::run_zahir_batch(&[abs.as_path()], ublx_opts)?;
        let map = integrations::get_zahir_output_by_path(&zr, Some(&dir_abs));
        let out = map
            .get(path_rel)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("ZahirScan produced no output for this path"))?;
        db_ops::update_snapshot_zahir_for_path(db_path, dir_to_ublx, path_rel, out)?;
        Ok::<(), anyhow::Error>(())
    })();

    if let Err(e) = db_ops::UblxCleanup::delete_ublx_tmp_files(dir_to_ublx) {
        log::debug!("remove ublx tmp after enhance: {e}");
    }
    result
}
