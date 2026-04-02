//! Background on-disk exports (Zahir JSON, lens Markdown): worker thread + toasts from Command Mode.

use std::path::PathBuf;
use std::sync::mpsc;

use log::Level;

use crate::app::RunUblxParams;
use crate::config::UBLX_NAMES;
use crate::engine::db_ops;
use crate::layout::setup::UblxState;
use crate::ui::{UI_STRINGS, show_operation_toast};

// --- Zahir JSON (`Ctrl+A` `x`) ------------------------------------------------

/// Drain a completed Zahir export and show a toast.
pub fn zahir_poll_and_finish(state: &mut UblxState, params: &mut RunUblxParams<'_>) {
    let Some(rx) = params.zahir_export_rx.as_ref() else {
        return;
    };
    let Ok(result) = rx.try_recv() else {
        return;
    };
    params.zahir_export_rx = None;
    finish_export_poll(state, params, result, &ExportPollToasts::zahir());
}

/// Spawn [`db_ops::export_zahir_json_flat`] if requested and no job is in flight.
///
/// # Panics
///
/// If [`std::thread::Builder::spawn`] fails.
pub fn zahir_spawn_if_requested(state: &mut UblxState, params: &mut RunUblxParams<'_>) {
    if !state.zahir_export_load.requested || params.zahir_export_rx.is_some() {
        return;
    }
    let dir_to_ublx = params.dir_to_ublx.clone();
    let db_path = params.db_path.clone();
    spawn_export_job(
        &mut state.zahir_export_load.requested,
        &mut params.zahir_export_rx,
        dir_to_ublx,
        db_path,
        "ublx-zahir-export",
        |dir_to_ublx, db_path| db_ops::export_zahir_json_flat(&dir_to_ublx, &db_path),
    );
}

// --- Lens Markdown (`Ctrl+A` `l`) --------------------------------------------

/// Drain a completed lens export and show a toast.
pub fn lens_poll_and_finish(state: &mut UblxState, params: &mut RunUblxParams<'_>) {
    let Some(rx) = params.lens_export_rx.as_ref() else {
        return;
    };
    let Ok(result) = rx.try_recv() else {
        return;
    };
    params.lens_export_rx = None;
    finish_export_poll(state, params, result, &ExportPollToasts::lenses());
}

/// Spawn [`db_ops::export_lenses_markdown_flat`] if requested and no job is in flight.
///
/// # Panics
///
/// If [`std::thread::Builder::spawn`] fails.
pub fn lens_spawn_if_requested(state: &mut UblxState, params: &mut RunUblxParams<'_>) {
    if !state.lens_export_load.requested || params.lens_export_rx.is_some() {
        return;
    }
    let dir_to_ublx = params.dir_to_ublx.clone();
    let db_path = params.db_path.clone();
    spawn_export_job(
        &mut state.lens_export_load.requested,
        &mut params.lens_export_rx,
        dir_to_ublx,
        db_path,
        "ublx-lens-export",
        |dir_to_ublx, db_path| db_ops::export_lenses_markdown_flat(&dir_to_ublx, &db_path),
    );
}

// --- Shared -------------------------------------------------------------------

struct ExportPollToasts {
    folder_name: &'static str,
    none_msg: &'static str,
    ok_msg_with_n: &'static str,
    failed_prefix: &'static str,
    op_name: &'static str,
}

impl ExportPollToasts {
    fn zahir() -> Self {
        Self {
            folder_name: UBLX_NAMES.zahir_export_dir_name,
            none_msg: UI_STRINGS.toasts.export_zahir_none,
            ok_msg_with_n: UI_STRINGS.toasts.export_zahir_ok,
            failed_prefix: UI_STRINGS.toasts.export_zahir_failed_prefix,
            op_name: "zahir-exporter",
        }
    }

    fn lenses() -> Self {
        Self {
            folder_name: UBLX_NAMES.lens_export_dir_name,
            none_msg: UI_STRINGS.toasts.export_lenses_none,
            ok_msg_with_n: UI_STRINGS.toasts.export_lenses_ok,
            failed_prefix: UI_STRINGS.toasts.export_lenses_failed_prefix,
            op_name: "lens-markdown-exporter",
        }
    }
}

fn spawn_export_job(
    gate_requested: &mut bool,
    rx_slot: &mut Option<mpsc::Receiver<Result<usize, String>>>,
    dir_to_ublx: PathBuf,
    db_path: PathBuf,
    thread_name: &'static str,
    job: impl FnOnce(PathBuf, PathBuf) -> Result<usize, anyhow::Error> + Send + 'static,
) {
    *gate_requested = false;
    let (tx, rx) = mpsc::channel();
    *rx_slot = Some(rx);
    std::thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            let r = job(dir_to_ublx, db_path).map_err(|e| e.to_string());
            let _ = tx.send(r);
        })
        .unwrap_or_else(|e| panic!("failed to spawn `{thread_name}`: {e}"));
}

fn finish_export_poll(
    state: &mut UblxState,
    params: &mut RunUblxParams<'_>,
    result: Result<usize, String>,
    toasts: &ExportPollToasts,
) {
    match result {
        Ok(0) => show_operation_toast(state, params, toasts.none_msg, toasts.op_name, Level::Info),
        Ok(n) => {
            let msg = format!(
                "{} to {}/",
                toasts.ok_msg_with_n.replace("{N}", &n.to_string()),
                toasts.folder_name
            );
            show_operation_toast(state, params, msg, toasts.op_name, Level::Info);
        }
        Err(e) => {
            let msg = format!("{}{}", toasts.failed_prefix, e);
            show_operation_toast(state, params, msg, toasts.op_name, Level::Error);
        }
    }
}
