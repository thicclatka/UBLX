//! ublx-settings applet: first-tick toast and config file watcher reload.

use std::time::Duration;

use crate::config::{OPERATION_NAME, UblxOpts};
use crate::handlers::reload;
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::UblxState;
use crate::ui::consts::UI_STRINGS;
use crate::utils::notifications;

/// Window (ms) after we write config ourselves (e.g. theme selector) during which a file-watcher reload is treated as self-caused.
const CONFIG_SELF_WRITE_WINDOW_MS: u64 = 800;

/// Show ublx-settings toast on first tick (e.g. config loaded / validation message from startup).
pub fn on_first_tick(state: &mut UblxState, params: &RunUblxParams<'_>) {
    if !state.first_tick {
        return;
    }
    state.first_tick = false;
    if let Some(b) = params.bumper {
        notifications::show_toast_slot(
            &mut state.toasts.slots,
            b,
            Some(OPERATION_NAME.ublx_settings().as_str()),
            &mut state.toasts.consumed_per_operation,
        );
    }
}

/// If config watcher fired: optionally clear theme override (if external save), then apply reload and optional toast.
pub fn on_config_reload(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
) {
    let from_external_save = state
        .config_written_by_us_at
        .as_ref()
        .is_none_or(|t| t.elapsed() >= Duration::from_millis(CONFIG_SELF_WRITE_WINDOW_MS));
    if from_external_save {
        state.theme.override_name = None;
    }
    let reload_msg = if from_external_save {
        Some(UI_STRINGS.config_reload_triggered_by_save())
    } else {
        None
    };
    reload::apply_config_reload(params, ublx_opts, state, reload_msg);
}
