//! theme-selector applet: open selector, handle keys (Up/Down/Enter/Esc), apply theme and toast.

use std::path::PathBuf;

use crate::app::RunUblxParams;
use crate::config::{UblxOpts, UblxPaths, write_local_theme};
use crate::handlers::applets::settings;
use crate::layout::setup::UblxState;
use crate::themes;
use crate::ui::{keymap::UblxAction, show_operation_toast};
use crate::utils::clamp_selection;

/// Context for the theme selector: indexed project dir (local `ublx.toml` / `.ublx.toml`) + current theme label.
#[derive(Clone, Debug)]
pub struct ThemeContext {
    /// Always write theme via [`write_local_theme`] for this indexed directory.
    pub project_dir: PathBuf,
    pub current_theme_name: Option<String>,
}

/// Build selector context. Theme is always persisted to **local** project config, never global-only.
#[must_use]
pub fn context_from_state(
    _state_ref: &UblxState,
    params_ref: &RunUblxParams<'_>,
    display_theme_name: Option<&str>,
) -> ThemeContext {
    ThemeContext {
        project_dir: params_ref.dir_to_ublx.to_path_buf(),
        current_theme_name: display_theme_name.map(String::from),
    }
}

/// Open the theme selector (set state; caller should return after so no other action runs).
pub fn open(state_mut: &mut UblxState, ctx_ref: &ThemeContext) {
    let current = ctx_ref
        .current_theme_name
        .as_deref()
        .or(state_mut.theme.override_name.as_deref());
    state_mut.theme.before_selector = current.map(String::from);
    state_mut.theme.selector_index = themes::theme_ordered_list()
        .iter()
        .position(|t| current == Some(t.name))
        .unwrap_or(0);
    state_mut.theme.selector_visible = true;
}

/// Handle one key while theme selector is visible. Caller should return after (no further action handling).
pub fn handle_key(
    state_mut: &mut UblxState,
    params_mut: &mut RunUblxParams<'_>,
    ublx_opts_mut: &mut UblxOpts,
    ctx: ThemeContext,
    action: UblxAction,
) {
    let opts = themes::theme_ordered_list();
    let n = opts.len();
    match action {
        UblxAction::Quit | UblxAction::SearchClear => {
            state_mut.theme.override_name = state_mut.theme.before_selector.clone();
            state_mut.theme.selector_visible = false;
        }
        UblxAction::MoveDown => {
            state_mut.theme.selector_index = clamp_selection(state_mut.theme.selector_index + 1, n);
        }
        UblxAction::MoveUp => {
            state_mut.theme.selector_index =
                clamp_selection(state_mut.theme.selector_index.saturating_sub(1), n);
        }
        UblxAction::SearchSubmit => {
            let display_name = opts[state_mut.theme.selector_index].name;
            write_local_theme(&UblxPaths::new(ctx.project_dir.as_path()), display_name);
            state_mut.config_written_by_us_at = Some(std::time::Instant::now());
            settings::apply_config_reload(params_mut, ublx_opts_mut, state_mut, None::<&str>);
            let theme_msg = format!("Changed theme to {display_name}");
            show_operation_toast(
                state_mut,
                params_mut,
                theme_msg,
                "theme-selector",
                log::Level::Info,
            );
            state_mut.theme.override_name = Some(display_name.to_string());
            state_mut.theme.selector_visible = false;
        }
        _ => {}
    }
}
