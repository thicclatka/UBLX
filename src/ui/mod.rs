pub mod consts;
pub mod input;
pub mod keymap;
pub mod lens;
pub mod menu;
pub mod snapshot;

use crate::config::OPERATION_NAME;
use crate::layout::{event_loop::RunUblxParams, setup::UblxState};
use crate::utils::notifications;

pub use consts::*;

/// Push a message to the bumper and refresh the stacked toast. `operation_name_suffix` is passed to `OPERATION_NAME.op` (e.g. `"lens"` → `ublx: lens`). No-op if `params.bumper` is None.
pub fn show_operation_toast(
    state: &mut UblxState,
    params: &RunUblxParams<'_>,
    message: impl AsRef<str>,
    operation_name_suffix: &str,
    level: log::Level,
) {
    let op = OPERATION_NAME.op(operation_name_suffix);
    if let Some(b) = params.bumper {
        let msg = message.as_ref();
        b.push_with_operation(level, msg, Some(op.as_str()));
        notifications::show_toast_slot(
            &mut state.toasts.slots,
            b,
            Some(op.as_str()),
            &mut state.toasts.consumed_per_operation,
        );
    }
}
