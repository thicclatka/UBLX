//! theme-selector applet: open selector, handle keys (Up/Down/Enter/Esc), apply theme and toast.

use std::path::Path;

use crate::config::{OPERATION_NAME, UblxPaths, write_local_theme};
use crate::layout::event_loop::RunUblxParams;
use crate::layout::setup::UblxState;
use crate::layout::themes;
use crate::ui::keymap::UblxAction;
use crate::utils::{format::clamp_selection, notifications::show_toast_slot};

/// Theme context for the selector: (dir for local config, current theme name for preview/revert).
pub type ThemeContext<'a> = Option<(&'a Path, Option<&'a str>)>;

/// Open the theme selector (set state; caller should return after so no other action runs).
pub fn open(state: &mut UblxState, theme_ctx: ThemeContext<'_>) {
    let current = theme_ctx
        .and_then(|(_, t)| t)
        .or(state.theme.override_name.as_deref());
    state.theme.before_selector = current.map(String::from);
    state.theme.selector_index = themes::theme_options()
        .iter()
        .position(|o| current == Some(o.display_name))
        .unwrap_or(0);
    state.theme.selector_visible = true;
}

/// Handle one key while theme selector is visible. Caller should return after (no further action handling).
pub fn handle_key(
    state: &mut UblxState,
    params: &RunUblxParams<'_>,
    theme_ctx: ThemeContext<'_>,
    action: UblxAction,
) {
    let opts = themes::theme_options();
    let n = opts.len();
    match action {
        UblxAction::Quit | UblxAction::SearchClear => {
            state.theme.override_name = state.theme.before_selector.clone();
            state.theme.selector_visible = false;
        }
        UblxAction::MoveDown => {
            state.theme.selector_index = clamp_selection(state.theme.selector_index + 1, n);
        }
        UblxAction::MoveUp => {
            state.theme.selector_index =
                clamp_selection(state.theme.selector_index.saturating_sub(1), n);
        }
        UblxAction::SearchSubmit => {
            let display_name = opts[state.theme.selector_index].display_name;
            if let Some((dir, _)) = theme_ctx {
                write_local_theme(&UblxPaths::new(dir), display_name);
                state.config_written_by_us_at = Some(std::time::Instant::now());
            }
            if let Some(b) = params.bumper {
                let theme_msg = format!("Changed theme to {display_name}");
                b.push_with_operation(
                    log::Level::Info,
                    theme_msg.as_str(),
                    Some(OPERATION_NAME.theme_selector()),
                );
                show_toast_slot(
                    &mut state.toasts.slots,
                    b,
                    Some(OPERATION_NAME.theme_selector()),
                    &mut state.toasts.consumed_per_operation,
                );
            }
            state.theme.override_name = Some(display_name.to_string());
            state.theme.selector_visible = false;
        }
        _ => {}
    }
}
