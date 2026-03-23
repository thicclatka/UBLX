//! Duplicate detection: group snapshot rows by content (hash from DB or computed from file).

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use blake3::Hasher;
use rayon::prelude::*;

use crate::utils::path::resolve_under_root;

use super::{SnapshotPathSizeHash, load_snapshot_path_size_hash};

/// One group of duplicate paths (same content). Left panel shows one name per group.
#[derive(Clone, Debug)]
pub struct DuplicateGroup {
    pub paths: Vec<String>,
}

impl DuplicateGroup {
    /// Representative name for the left panel (one name per duplicate).
    pub fn representative_name(&self) -> &str {
        self.paths
            .iter()
            .min_by_key(|p| p.len())
            .map_or("", String::as_str)
    }
}

const CONTENT_HASH_CHUNK: usize = 64 * 1024;
const MAX_FILE_SIZE_FOR_CONTENT_HASH: u64 = 50 * 1024 * 1024; // 50 MiB cap for no-hash path

/// Compute blake3 hash of file at `full_path`. Returns None on read error or if too large.
fn content_hash(full_path: &Path, size: u64) -> Option<[u8; 32]> {
    if size > MAX_FILE_SIZE_FOR_CONTENT_HASH {
        return None;
    }
    let mut f = fs::File::open(full_path).ok()?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; CONTENT_HASH_CHUNK];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize().into())
}

/// Group rows by 32-byte hash; keep only groups with more than one path.
fn group_by_hash(rows: &[SnapshotPathSizeHash]) -> Vec<DuplicateGroup> {
    let mut by_hash: HashMap<[u8; 32], Vec<String>> = HashMap::new();
    for (path, _size, hash_opt) in rows {
        let Some(blob) = hash_opt.as_ref() else {
            continue;
        };
        if blob.len() != 32 {
            continue;
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(blob);
        by_hash.entry(key).or_default().push(path.clone());
    }
    by_hash
        .into_values()
        .filter(|paths| paths.len() > 1)
        .map(|paths| DuplicateGroup { paths })
        .collect()
}

/// When DB has no hashes: group by size, then for each size bucket with >1 file compute content hash in parallel and group.
fn group_by_content_hash(rows: &[SnapshotPathSizeHash], dir_to_ublx: &Path) -> Vec<DuplicateGroup> {
    let by_size: HashMap<u64, Vec<(String, u64)>> = rows
        .iter()
        .map(|(path, size, _)| (path.clone(), *size))
        .fold(HashMap::new(), |mut m, (path, size)| {
            m.entry(size).or_default().push((path, size));
            m
        });
    let dir = dir_to_ublx.to_path_buf();
    let buckets: Vec<_> = by_size
        .into_values()
        .filter(|path_sizes| path_sizes.len() >= 2)
        .collect();
    buckets
        .par_iter()
        .flat_map(|path_sizes| {
            let hashed: Vec<(String, [u8; 32])> = path_sizes
                .par_iter()
                .filter_map(|(path, size)| {
                    let full = resolve_under_root(&dir, path);
                    content_hash(&full, *size).map(|h| (path.clone(), h))
                })
                .collect();
            let mut by_hash: HashMap<[u8; 32], Vec<String>> = HashMap::new();
            for (path, key) in hashed {
                by_hash.entry(key).or_default().push(path);
            }
            by_hash
                .into_values()
                .filter(|p| p.len() > 1)
                .map(|paths| DuplicateGroup { paths })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Load duplicate groups from the snapshot. If any row has a hash, group by DB hash; otherwise group by content hash (read files).
/// Returns empty vec when no duplicates exist or on error.
///
/// # Errors
///
/// Returns [`anyhow::Error`] on `SQLite` errors, or when reading file contents for hashing fails.
pub fn load_duplicate_groups(
    db_path: &Path,
    dir_to_ublx: &Path,
) -> Result<Vec<DuplicateGroup>, anyhow::Error> {
    let rows = load_snapshot_path_size_hash(db_path)?;
    let any_has_hash = rows
        .iter()
        .any(|(_, _, h)| h.as_ref().is_some_and(|b| b.len() == 32));
    let groups = if any_has_hash {
        group_by_hash(&rows)
    } else {
        group_by_content_hash(&rows, dir_to_ublx)
    };
    Ok(groups)
}
