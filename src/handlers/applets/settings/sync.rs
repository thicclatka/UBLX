//! ublx-settings applet: first-tick toast and config file watcher reload.

use std::time::Duration;

use crate::app::RunUblxParams;
use crate::config::{OPERATION_NAME, UblxOpts, UblxPaths, first_validation_error_message};
use crate::layout::setup::UblxState;
use crate::themes;
use crate::ui::consts::UI_STRINGS;
use crate::ui::show_operation_toast;
use crate::ui::snapshot::show_force_full_enhance_started_toast;
use crate::utils;

/// Window (ms) after we write config ourselves (e.g. theme selector) during which a file-watcher reload is treated as self-caused.
const CONFIG_SELF_WRITE_WINDOW_MS: u64 = 800;

/// Show ublx-settings toast on first tick (e.g. config loaded / validation message from startup).
pub fn on_first_tick(state_mut: &mut UblxState, params_ref: &RunUblxParams<'_>) {
    if !state_mut.session.tick.first_tick {
        return;
    }
    state_mut.session.tick.first_tick = false;
    if let Some(b) = params_ref.bumper {
        let op = OPERATION_NAME.op("settings");
        utils::show_toast_slot(
            &mut state_mut.toasts.slots,
            b,
            Some(op.as_str()),
            &mut state_mut.toasts.consumed_per_operation,
        );
    }
}

/// Copy theme / layout from [`UblxOpts`] into [`RunUblxParams`] after reload.
pub fn sync_run_params_from_opts(params_mut: &mut RunUblxParams<'_>, ublx_opts_ref: &UblxOpts) {
    params_mut.theme.clone_from(&ublx_opts_ref.theme);
    params_mut.layout.clone_from(&ublx_opts_ref.layout);
}

/// If config watcher fired: optionally clear theme override (if external save), then apply reload and optional toast.
pub fn on_config_reload(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
) {
    let from_external_save = state_mut
        .config_written_by_us_at
        .as_ref()
        .is_none_or(|t| t.elapsed() >= Duration::from_millis(CONFIG_SELF_WRITE_WINDOW_MS));
    if from_external_save {
        state_mut.theme.override_name = None;
    }
    apply_config_reload(
        params_mut,
        ublx_opts_mut,
        state_mut,
        Some(UI_STRINGS.toasts.config_reloaded),
    );
}

/// Reloads hot-reloadable config from paths and syncs theme/layout into params. Validates before applying; on validation failure shows a toast with variable-specific errors. If applied and `message` is `Some`, shows success toast (use `None` when the change was caused by us, e.g. theme selector write).
pub fn apply_config_reload(
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    state_mut: &mut UblxState,
    message: Option<impl AsRef<str>>,
) {
    let paths = UblxPaths::new(params_mut.dir_to_ublx);
    let valid_themes: Vec<&str> = themes::theme_ordered_list()
        .iter()
        .map(|t| t.name)
        .collect();
    let old_enable_enhance_all = ublx_opts_mut.enable_enhance_all;
    let result = ublx_opts_mut.reload_hot_config(&paths, &valid_themes);

    if result.applied {
        sync_run_params_from_opts(params_mut, ublx_opts_mut);
        if !old_enable_enhance_all && ublx_opts_mut.enable_enhance_all {
            ublx_opts_mut.enable_enhance_all_cache_before_apply = Some(false);
            if params_mut.startup.defer_first_snapshot {
                if !state_mut.session.reload.force_full_enhance_toast_shown {
                    params_mut.startup.pending_force_full_enhance_toast = true;
                }
            } else {
                schedule_snapshot_after_enable_enhance_flip(state_mut);
                show_force_full_enhance_started_toast(state_mut, params_mut);
            }
        }
        if let Some(msg) = message {
            show_operation_toast(state_mut, params_mut, msg, "settings", log::Level::Info);
        }
    } else if !result.validation_errors.is_empty() {
        sync_run_params_from_opts(params_mut, ublx_opts_mut);
        let msg = first_validation_error_message(&result.validation_errors);
        let warn_msg = format!("Config validation: {msg}");
        show_operation_toast(
            state_mut,
            params_mut,
            warn_msg,
            "settings",
            log::Level::Warn,
        );
    }
}

/// Queue a background snapshot after `enable_enhance_all` flips to `true` on hot reload. If a snapshot is already running, run again when it finishes.
fn schedule_snapshot_after_enable_enhance_flip(state_mut: &mut UblxState) {
    if state_mut.snapshot_bg.done_received {
        state_mut.snapshot_bg.requested = true;
    } else {
        state_mut.snapshot_bg.defer_snapshot_after_current = true;
    }
}
