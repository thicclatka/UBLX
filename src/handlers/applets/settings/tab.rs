//! Settings tab: enter, key dispatch (`handle_key`), and orchestration. Bool rows and layout edit live in
//! [`super::bool_rows`] and [`super::layout_edit`]; path/overlay sync in [`super::context`].

use crate::app::RunUblxParams;
use crate::config::{
    LayoutOverlay, UblxOpts, UblxOverlay, UblxPaths, load_ublx_toml, write_ublx_overlay_at,
};
use crate::handlers::state_transitions::PREVIEW_SCROLL_STEP_LINES;
use crate::layout::setup::{SettingsConfigScope, UblxState};
use crate::ui::{keymap::UblxAction, show_operation_toast};
use crate::utils::clamp_selection;

use super::apply_config_reload;
use super::bool_rows::{bool_row_count, bool_row_label, overlay_bool, write_bool};
use super::context::{
    layout_overlay_for_local_editing, local_edit_context, refresh_editing_metadata,
    sync_layout_buffers_from_overlay,
};
use super::layout_edit::{
    bump_settings_cursor, layout_button_index, left_cursor_len, max_left_cursor,
    parse_layout_triplet,
};

pub fn on_enter_settings(state_mut: &mut UblxState, params_ref: &RunUblxParams<'_>) {
    state_mut.settings.left_cursor = 0;
    state_mut.settings.right_scroll = 0;
    state_mut.settings.layout_unlocked = false;
    refresh_editing_metadata(state_mut, params_ref);
    let scope = state_mut.settings.scope;
    state_mut.settings.left_cursor = clamp_selection(
        state_mut.settings.left_cursor,
        left_cursor_len(&state_mut.settings, scope),
    );
}

/// Enter on Search: toggle bool row, or layout row (unlock / save).
fn handle_settings_search_submit(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    scope: SettingsConfigScope,
) {
    let btn = layout_button_index(scope);
    let cur = state_mut.settings.left_cursor;
    if cur < bool_row_count(scope) {
        let Some(path) = state_mut.settings.editing_path.clone() else {
            return;
        };
        let paths = UblxPaths::new(params_mut.dir_to_ublx);
        let mut overlay = load_ublx_toml(Some(path.clone()), None).unwrap_or_default();
        let merged_before = match scope {
            SettingsConfigScope::Local => {
                let global_o = load_ublx_toml(paths.global_config(), None);
                UblxOverlay::merge(global_o, Some(overlay.clone()))
            }
            SettingsConfigScope::Global => overlay.clone(),
        };
        let v = !overlay_bool(&merged_before, scope, cur);
        write_bool(&mut overlay, scope, cur, v);
        write_ublx_overlay_at(&path, &overlay);
        state_mut.config_written_by_us_at = Some(std::time::Instant::now());
        apply_config_reload(params_mut, ublx_opts_mut, state_mut, None::<&str>);
        refresh_editing_metadata(state_mut, params_mut);
        let label = bool_row_label(scope, cur, false);
        let msg = format!("{label} = {v}");
        show_operation_toast(
            state_mut,
            params_mut,
            msg,
            "settings-bool",
            log::Level::Info,
        );
    } else if cur == btn {
        if state_mut.settings.layout_unlocked {
            let Some(path) = state_mut.settings.editing_path.clone() else {
                return;
            };
            let Some((l, m, r)) = parse_layout_triplet(
                &state_mut.settings.layout_left_buf,
                &state_mut.settings.layout_mid_buf,
                &state_mut.settings.layout_right_buf,
            ) else {
                show_operation_toast(
                    state_mut,
                    params_mut,
                    "layout: three u8 must sum to 100",
                    "settings-layout",
                    log::Level::Warn,
                );
                return;
            };
            let mut overlay = load_ublx_toml(Some(path.clone()), None).unwrap_or_default();
            overlay.layout = Some(LayoutOverlay {
                left_pct: l,
                middle_pct: m,
                right_pct: r,
            });
            write_ublx_overlay_at(&path, &overlay);
            state_mut.config_written_by_us_at = Some(std::time::Instant::now());
            apply_config_reload(params_mut, ublx_opts_mut, state_mut, None::<&str>);
            state_mut.settings.layout_unlocked = false;
            refresh_editing_metadata(state_mut, params_mut);
            state_mut.settings.left_cursor =
                clamp_selection(btn, left_cursor_len(&state_mut.settings, scope));
            show_operation_toast(
                state_mut,
                params_mut,
                format!("layout {l}/{m}/{r}"),
                "settings-layout",
                log::Level::Info,
            );
        } else {
            state_mut.settings.layout_unlocked = true;
            let paths = UblxPaths::new(params_mut.dir_to_ublx);
            if scope == SettingsConfigScope::Local {
                let (local_o, merged) = local_edit_context(&paths);
                let lay_src = layout_overlay_for_local_editing(local_o.as_ref(), &merged);
                sync_layout_buffers_from_overlay(&mut state_mut.settings, lay_src);
            } else if let Some(path) = state_mut.settings.editing_path.clone()
                && let Some(overlay) = load_ublx_toml(Some(path), None)
            {
                sync_layout_buffers_from_overlay(&mut state_mut.settings, &overlay);
            }
            state_mut.settings.left_cursor =
                (btn + 1).min(max_left_cursor(&state_mut.settings, scope));
        }
    }
}

/// Handle a mapped action while on the Settings tab. Returns true if the key should not propagate.
#[must_use]
pub fn handle_key(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    action: UblxAction,
) -> bool {
    let scope = state_mut.settings.scope;

    match action {
        UblxAction::Tab => {
            state_mut.settings.scope = match state_mut.settings.scope {
                SettingsConfigScope::Global => SettingsConfigScope::Local,
                SettingsConfigScope::Local => SettingsConfigScope::Global,
            };
            state_mut.settings.layout_unlocked = false;
            refresh_editing_metadata(state_mut, params_mut);
            let sc = state_mut.settings.scope;
            state_mut.settings.left_cursor = clamp_selection(
                state_mut.settings.left_cursor,
                left_cursor_len(&state_mut.settings, sc),
            );
            true
        }
        UblxAction::ScrollPreviewUp => {
            state_mut.settings.right_scroll = state_mut
                .settings
                .right_scroll
                .saturating_sub(PREVIEW_SCROLL_STEP_LINES);
            true
        }
        UblxAction::ScrollPreviewDown => {
            state_mut.settings.right_scroll = state_mut
                .settings
                .right_scroll
                .saturating_add(PREVIEW_SCROLL_STEP_LINES);
            true
        }
        UblxAction::PreviewTop => {
            state_mut.settings.right_scroll = 0;
            true
        }
        UblxAction::PreviewBottom => {
            state_mut.settings.right_scroll = u16::MAX;
            true
        }
        UblxAction::MoveDown | UblxAction::MoveDownFast => {
            bump_settings_cursor(state_mut, scope, true);
            true
        }
        UblxAction::MoveUp | UblxAction::MoveUpFast => {
            bump_settings_cursor(state_mut, scope, false);
            true
        }
        UblxAction::FocusCategories
        | UblxAction::FocusContents
        | UblxAction::ListTop
        | UblxAction::ListBottom
        | UblxAction::CycleContentSort
        | UblxAction::CycleRightPane
        | UblxAction::RightPaneViewer
        | UblxAction::RightPaneTemplates
        | UblxAction::RightPaneMetadata
        | UblxAction::RightPaneWriting
        | UblxAction::ViewerFullscreenToggle => true,
        UblxAction::SearchSubmit => {
            handle_settings_search_submit(state_mut, params_mut, ublx_opts_mut, scope);
            true
        }
        _ => false,
    }
}
