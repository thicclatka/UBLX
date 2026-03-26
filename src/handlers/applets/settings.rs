//! ublx-settings applet: first-tick toast and config file watcher reload.

use std::time::Duration;

use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths, first_validation_error_message};
use crate::layout::{event_loop::RunUblxParams, setup::UblxState, themes};
use crate::ui::consts::UI_STRINGS;
use crate::ui::show_operation_toast;
use crate::ui::snapshot::show_force_full_enhance_started_toast;
use crate::utils::notifications;

/// Window (ms) after we write config ourselves (e.g. theme selector) during which a file-watcher reload is treated as self-caused.
const CONFIG_SELF_WRITE_WINDOW_MS: u64 = 800;

/// Show ublx-settings toast on first tick (e.g. config loaded / validation message from startup).
pub fn on_first_tick(state: &mut UblxState, params: &RunUblxParams<'_>) {
    if !state.session.tick.first_tick {
        return;
    }
    state.session.tick.first_tick = false;
    if let Some(b) = params.bumper {
        let op = OPERATION_NAME.op("settings");
        notifications::show_toast_slot(
            &mut state.toasts.slots,
            b,
            Some(op.as_str()),
            &mut state.toasts.consumed_per_operation,
        );
    }
}

/// Copy theme / layout / transparency from [`UblxOpts`] into [`RunUblxParams`] after reload.
pub fn sync_run_params_from_opts(params: &mut RunUblxParams<'_>, ublx_opts: &UblxOpts) {
    params.theme.clone_from(&ublx_opts.theme);
    params.display.transparent = ublx_opts.transparent;
    params.layout.clone_from(&ublx_opts.layout);
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
    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let old_enable_enhance_all = ublx_opts.enable_enhance_all;
    let result = ublx_opts.reload_hot_config(&paths, &valid_themes);

    if result.applied {
        params.theme.clone_from(&ublx_opts.theme);
        params.display.transparent = ublx_opts.transparent;
        params.layout = ublx_opts.layout.clone();
        if !old_enable_enhance_all && ublx_opts.enable_enhance_all {
            ublx_opts.enable_enhance_all_cache_before_apply = Some(false);
            if params.startup.defer_first_snapshot {
                if !state.session.reload.force_full_enhance_toast_shown {
                    params.startup.pending_force_full_enhance_toast = true;
                }
            } else {
                schedule_snapshot_after_enable_enhance_flip(state);
                show_force_full_enhance_started_toast(state, params);
            }
        }
        if let Some(msg) = message {
            show_operation_toast(state, params, msg, "settings", log::Level::Info);
        }
    } else if !result.validation_errors.is_empty() {
        params.theme.clone_from(&ublx_opts.theme);
        params.display.transparent = ublx_opts.transparent;
        params.layout = ublx_opts.layout.clone();
        let msg = first_validation_error_message(&result.validation_errors);
        let warn_msg = format!("Config validation: {msg}");
        show_operation_toast(state, params, warn_msg, "settings", log::Level::Warn);
    }
}

/// Queue a background snapshot after `enable_enhance_all` flips to `true` on hot reload. If a snapshot is already running, run again when it finishes.
fn schedule_snapshot_after_enable_enhance_flip(state: &mut UblxState) {
    if state.snapshot_bg.done_received {
        state.snapshot_bg.requested = true;
    } else {
        state.snapshot_bg.defer_snapshot_after_current = true;
    }
}
