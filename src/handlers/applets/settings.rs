//! ublx-settings applet: first-tick toast and config file watcher reload.

use std::time::Duration;

use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths, first_validation_error_message};
use crate::layout::{event_loop::RunUblxParams, setup::UblxState, themes};
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
    apply_config_reload(params, ublx_opts, state, reload_msg);
}

/// Reloads hot-reloadable config from paths and syncs theme/transparent/layout into params. Validates before applying; on validation failure shows a toast with variable-specific errors. If applied and `message` is `Some`, shows success toast (use `None` when the change was caused by us, e.g. theme selector write).
pub fn apply_config_reload(
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    state: &mut UblxState,
    message: Option<impl AsRef<str>>,
) {
    let paths = UblxPaths::new(params.dir_to_ublx);
    let valid_themes: Vec<&str> = themes::theme_options()
        .iter()
        .map(|o| o.display_name)
        .collect();
    let result = ublx_opts.reload_hot_config(&paths, &valid_themes);

    if result.applied {
        params.theme = ublx_opts.theme.clone();
        params.transparent = ublx_opts.transparent;
        params.layout = ublx_opts.layout.clone();
        if let Some(msg) = message {
            let op = OPERATION_NAME.ublx_settings();
            if let Some(b) = params.bumper {
                b.push_with_operation(log::Level::Info, msg.as_ref().to_string(), Some(op.clone()));
                notifications::show_toast_slot(
                    &mut state.toasts.slots,
                    b,
                    Some(op.as_str()),
                    &mut state.toasts.consumed_per_operation,
                );
            }
        }
    } else if !result.validation_errors.is_empty() {
        params.theme = ublx_opts.theme.clone();
        params.transparent = ublx_opts.transparent;
        params.layout = ublx_opts.layout.clone();
        let op = OPERATION_NAME.ublx_settings();
        if let Some(b) = params.bumper {
            let msg = first_validation_error_message(&result.validation_errors);
            b.push_with_operation(
                log::Level::Warn,
                format!("Config validation: {}", msg),
                Some(op.clone()),
            );
            notifications::show_toast_slot(
                &mut state.toasts.slots,
                b,
                Some(op.as_str()),
                &mut state.toasts.consumed_per_operation,
            );
        }
    }
}
