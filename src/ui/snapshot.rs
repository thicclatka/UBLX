//! Snapshot-related toasts (background index run, completion).

use crate::app::RunUblxParams;
use crate::config::OPERATION_NAME;
use crate::handlers::snapshot;
use crate::layout::setup::UblxState;
use crate::ui::consts::UI_STRINGS;
use crate::ui::show_operation_toast;
use crate::utils;

/// Toast when a force-full Zahir snapshot starts (e.g. `enable_enhance_all` just turned on).
pub fn show_force_full_enhance_started_toast(
    state_mut: &mut UblxState,
    params_ref: &RunUblxParams<'_>,
) {
    show_operation_toast(
        state_mut,
        params_ref,
        UI_STRINGS.toasts.force_full_enhance_background,
        "snapshot",
        log::Level::Info,
    );
}

/// After a background snapshot completes: bumper summary + stacked toast.
pub fn show_snapshot_completed_toast(
    state_mut: &mut UblxState,
    params_ref: &RunUblxParams<'_>,
    added: usize,
    mod_count: usize,
    removed: usize,
) {
    let Some(b) = params_ref.bumper else {
        return;
    };
    snapshot::push_snapshot_done_to_bumper(b, added, mod_count, removed);
    let op = OPERATION_NAME.op("snapshot");
    utils::show_toast_slot(
        &mut state_mut.toasts.slots,
        b,
        Some(op.as_str()),
        &mut state_mut.toasts.consumed_per_operation,
    );
}
