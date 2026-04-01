//! dupe-finder applet: request load, spawn background load, on result toast or switch to Duplicates tab.

use std::collections::HashSet;

use crate::app::RunUblxParams;
use crate::config::{OPERATION_NAME, UblxOpts};
use crate::engine::db_ops;
use crate::layout::setup;
use crate::ui::UI_STRINGS;
use crate::utils;

/// After reloading duplicate groups from the DB, drop ignore entries that no longer exist in any group.
pub fn prune_duplicate_ignores_after_reload(
    ignored: &mut HashSet<String>,
    groups: &[db_ops::DuplicateGroup],
) {
    let mut keep = HashSet::new();
    for g in groups {
        for p in &g.paths {
            keep.insert(p.clone());
        }
    }
    ignored.retain(|p| keep.contains(p));
}

/// Called when the background duplicate load returns. Empty → toast "No duplicates found"; non-empty → set groups and switch to Duplicates mode.
pub fn on_groups_received(
    state: &mut setup::UblxState,
    params: &mut RunUblxParams<'_>,
    groups: Vec<db_ops::DuplicateGroup>,
    mode: db_ops::DuplicateGroupingMode,
) {
    if groups.is_empty() {
        let op = OPERATION_NAME.op("dupe-finder");
        if let Some(b) = params.bumper {
            b.push_with_operation(log::Level::Info, UI_STRINGS.toasts.no_duplicates, Some(&op));
            utils::show_toast_slot(
                &mut state.toasts.slots,
                b,
                Some(op.as_str()),
                &mut state.toasts.consumed_per_operation,
            );
        }
    } else {
        params.duplicate_groups = groups;
        params.duplicate_mode = mode;
        state.duplicate_ignored_paths.clear();
        state.main_mode = setup::MainMode::Duplicates;
    }
}

/// If duplicate load was requested and no load is in progress, spawn the background load and clear the request flag.
pub fn spawn_if_requested(
    state: &mut setup::UblxState,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &UblxOpts,
) {
    if !state.duplicate_load.requested
        || !params.duplicate_groups.is_empty()
        || params.duplicate_groups_rx.is_some()
    {
        return;
    }
    state.duplicate_load.requested = false;
    let db_path = params.db_path.clone();
    let dir_to_ublx = params.dir_to_ublx.clone();
    let config_wants_hash = ublx_opts.nefax.with_hash;
    let (tx, rx) = std::sync::mpsc::channel();
    params.duplicate_groups_rx = Some(rx);
    std::thread::spawn(move || {
        let (groups, mode) =
            db_ops::load_duplicate_groups(&db_path, &dir_to_ublx, config_wants_hash)
                .unwrap_or((Vec::new(), db_ops::DuplicateGroupingMode::Hash));
        let _ = tx.send((groups, mode));
    });
}
