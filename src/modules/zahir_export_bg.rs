//! Background flat Zahir JSON export (Command Mode + `x`; same as `ublx --export`).

use crate::app::RunUblxParams;
use crate::config::UBLX_NAMES;
use crate::engine::db_ops;
use crate::layout::setup::UblxState;
use crate::ui::{UI_STRINGS, show_operation_toast};
use log::Level;

const EXPORT_OP_NAME: &str = "zahir-exporter";

/// Drain a completed export and show a toast; then see [`spawn_if_requested`].
pub fn poll_and_finish(state: &mut UblxState, params: &mut RunUblxParams<'_>) {
    let Some(rx) = params.zahir_export_rx.as_ref() else {
        return;
    };
    let Ok(result) = rx.try_recv() else {
        return;
    };
    params.zahir_export_rx = None;
    let folder = UBLX_NAMES.export_folder_name;
    match result {
        Ok(0) => {
            show_operation_toast(
                state,
                params,
                UI_STRINGS.toasts.export_zahir_none,
                EXPORT_OP_NAME,
                Level::Info,
            );
        }
        Ok(n) => {
            let msg = format!(
                "{} to {}/",
                UI_STRINGS
                    .toasts
                    .export_zahir_ok
                    .replace("{N}", &n.to_string()),
                folder
            );
            show_operation_toast(state, params, msg, EXPORT_OP_NAME, Level::Info);
        }
        Err(e) => {
            let msg = format!("{}{}", UI_STRINGS.toasts.export_zahir_failed_prefix, e);
            show_operation_toast(state, params, msg, EXPORT_OP_NAME, Level::Error);
        }
    }
}

/// If the user requested export and no job is in flight, spawn [`db_ops::export_zahir_json_flat`] on a worker thread.
///
/// # Panics
///
/// Thread fails to spawn.
pub fn spawn_if_requested(state: &mut UblxState, params: &mut RunUblxParams<'_>) {
    if !state.zahir_export_load.requested || params.zahir_export_rx.is_some() {
        return;
    }
    state.zahir_export_load.requested = false;
    let db_path = params.db_path.clone();
    let dir_to_ublx = params.dir_to_ublx.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    params.zahir_export_rx = Some(rx);
    std::thread::Builder::new()
        .name("ublx-zahir-export".into())
        .spawn(move || {
            let r =
                db_ops::export_zahir_json_flat(&dir_to_ublx, &db_path).map_err(|e| e.to_string());
            let _ = tx.send(r);
        })
        .expect("ublx-zahir-export thread");
}
