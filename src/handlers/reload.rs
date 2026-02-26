//! Config hot-reload: apply overlay from disk and optionally show toast.

use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths};
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::UblxState;
use crate::utils::notifications;

/// Reloads hot-reloadable config from paths and syncs theme/transparent into params. If something was applied and `message` is `Some`, pushes that message and shows the ublx-settings toast (use `None` when the change was caused by us, e.g. theme selector write).
pub fn apply_config_reload(
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    state: &mut UblxState,
    message: Option<impl AsRef<str>>,
) {
    let paths = UblxPaths::new(params.dir_to_ublx);
    let applied = ublx_opts.reload_hot_config(&paths);
    params.theme = ublx_opts.theme.clone();
    params.transparent = ublx_opts.transparent;
    params.layout = ublx_opts.layout.clone();
    if applied && let Some(msg) = message {
        let op = OPERATION_NAME.ublx_settings();
        if let Some(b) = params.bumper {
            b.push_with_operation(log::Level::Info, msg.as_ref().to_string(), Some(op.clone()));
            notifications::show_toast_slot(
                &mut state.toast_slots,
                b,
                Some(op.as_str()),
                &mut state.toast_consumed_per_operation,
            );
        }
    }
}
