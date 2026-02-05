use nefaxer::*;
use std::path::Path;

use crate::config::UblxOpts;

pub type NefaxEntry = nefaxer::Entry;
pub type NefaxResult = nefaxer::Nefax;
pub type NefaxDiff = nefaxer::Diff;
pub type NefaxPathMeta = nefaxer::PathMeta;

fn get_important_opts_from_nefaxer(dir: &Path) -> NefaxOpts {
    let (num_threads, drive_type, use_parallel_walk) = tuning_for_path(dir, None);
    NefaxOpts {
        num_threads: Some(num_threads),
        drive_type: Some(drive_type),
        use_parallel_walk: Some(use_parallel_walk),
        ..NefaxOpts::default()
    }
}

fn add_exclude_to_nefax_opts(opts: &mut NefaxOpts, exclude: &[String]) {
    opts.exclude.extend(exclude.iter().map(|s| s.to_string()));
}

pub fn pre_opts_for_nefaxer(dir: &Path, exclude: &[String]) -> NefaxOpts {
    let mut opts = get_important_opts_from_nefaxer(dir);
    add_exclude_to_nefax_opts(&mut opts, exclude);
    opts
}

fn extract_nefax_opts_from_ublx_opts(opts: &UblxOpts) -> NefaxOpts {
    opts.nefax_opts_with_workers()
}

pub fn run_nefaxer<F>(
    dir: &Path,
    ublx_opts: &UblxOpts,
    nefax: Option<&Nefax>,
    entry_callback: Option<F>,
) -> Result<(NefaxResult, NefaxDiff)>
where
    F: FnMut(&NefaxEntry),
{
    let nefax_opts = extract_nefax_opts_from_ublx_opts(ublx_opts);
    nefax_dir(dir, &nefax_opts, nefax, entry_callback)
}
