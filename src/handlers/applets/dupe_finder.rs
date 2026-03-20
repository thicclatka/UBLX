//! dupe-finder applet: request load, spawn background load, on result toast or switch to Duplicates tab.

use crate::config::OPERATION_NAME;
use crate::engine::db_ops;
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup;
use crate::ui::consts::UI_STRINGS;
use crate::utils::notifications;

/// Called when the background duplicate load returns. Empty → toast "No duplicates found"; non-empty → set groups and switch to Duplicates mode.
pub fn on_groups_received(
    state: &mut setup::UblxState,
    params: &mut RunUblxParams<'_>,
    groups: Vec<db_ops::DuplicateGroup>,
) {
    if groups.is_empty() {
        let op = OPERATION_NAME.op("dupe-finder");
        if let Some(b) = params.bumper {
            b.push_with_operation(
                log::Level::Info,
                UI_STRINGS.no_duplicates,
                Some(&op),
            );
            notifications::show_toast_slot(
                &mut state.toasts.slots,
                b,
                Some(op.as_str()),
                &mut state.toasts.consumed_per_operation,
            );
        }
    } else {
        params.duplicate_groups = groups;
        state.main_mode = setup::MainMode::Duplicates;
    }
}

/// If duplicate load was requested and no load is in progress, spawn the background load and clear the request flag.
pub fn spawn_if_requested(state: &mut setup::UblxState, params: &mut RunUblxParams<'_>) {
    if !state.duplicate_load.requested
        || !params.duplicate_groups.is_empty()
        || params.duplicate_groups_rx.is_some()
    {
        return;
    }
    state.duplicate_load.requested = false;
    let db_path = params.db_path.to_path_buf();
    let dir_to_ublx = params.dir_to_ublx.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    params.duplicate_groups_rx = Some(rx);
    std::thread::spawn(move || {
        let groups = db_ops::load_duplicate_groups(&db_path, &dir_to_ublx).unwrap_or_default();
        let _ = tx.send(groups);
    });
}
