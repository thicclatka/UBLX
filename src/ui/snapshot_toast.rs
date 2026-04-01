//! Snapshot-related toasts (background index run, completion).

use crate::app::RunUblxParams;
use crate::config::OPERATION_NAME;
use crate::handlers;
use crate::layout::setup::UblxState;
use crate::ui::consts::UI_STRINGS;
use crate::ui::show_operation_toast;
use crate::utils;

/// Toast when a force-full Zahir snapshot starts (cached `enable_enhance_all` was false; next snapshot runs full index-time Zahir).
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
    let op = OPERATION_NAME.op("snapshot");
    handlers::push_snapshot_done_to_bumper(b, added, mod_count, removed);
    // Bumper no longer stacks old snapshot lines; reset consumed so the toast shows the new pair.
    state_mut.toasts.consumed_per_operation.remove(&op);
    utils::show_toast_slot(
        &mut state_mut.toasts.slots,
        b,
        Some(op.as_str()),
        &mut state_mut.toasts.consumed_per_operation,
    );
}
