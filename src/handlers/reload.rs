//! Config hot-reload: apply overlay from disk and optionally show toast.

use crate::config::{first_validation_error_message, OPERATION_NAME, UblxOpts, UblxPaths};
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::UblxState;
use crate::layout::themes;
use crate::utils::notifications;

/// Reloads hot-reloadable config from paths and syncs theme/transparent/layout into params. Validates before applying; on validation failure shows a toast with variable-specific errors. If applied and `message` is `Some`, shows success toast (use `None` when the change was caused by us, e.g. theme selector write).
pub fn apply_config_reload(
    params: &mut RunUblxParams<'_>,
    ublx_opts: &mut UblxOpts,
    state: &mut UblxState,
    message: Option<impl AsRef<str>>,
) {
    let paths = UblxPaths::new(params.dir_to_ublx);
    let valid_themes: Vec<&str> = themes::theme_options().iter().map(|o| o.display_name).collect();
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
                    &mut state.toast_slots,
                    b,
                    Some(op.as_str()),
                    &mut state.toast_consumed_per_operation,
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
                &mut state.toast_slots,
                b,
                Some(op.as_str()),
                &mut state.toast_consumed_per_operation,
            );
        }
    }
}
