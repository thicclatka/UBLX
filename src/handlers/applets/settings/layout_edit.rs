//! Layout triplet (left/middle/right %) editing: cursor bounds, digit buffers, parse.

use crossterm::event::{KeyCode, KeyModifiers};

use super::bool_rows::bool_row_count;
use crate::layout::setup::{SettingsConfigScope, SettingsPaneState, UblxState};
use crate::utils::clamp_selection;

pub fn layout_button_index(scope: SettingsConfigScope) -> usize {
    bool_row_count(scope)
}

pub fn max_left_cursor(settings_ref: &SettingsPaneState, scope: SettingsConfigScope) -> usize {
    let b = layout_button_index(scope);
    if settings_ref.layout_unlocked {
        b + 3
    } else {
        b
    }
}

pub fn left_cursor_len(settings_ref: &SettingsPaneState, scope: SettingsConfigScope) -> usize {
    max_left_cursor(settings_ref, scope) + 1
}

pub fn bump_settings_cursor(state_mut: &mut UblxState, scope: SettingsConfigScope, down: bool) {
    let n = left_cursor_len(&state_mut.settings, scope);
    if down {
        state_mut.settings.left_cursor = clamp_selection(state_mut.settings.left_cursor + 1, n);
    } else {
        state_mut.settings.left_cursor =
            clamp_selection(state_mut.settings.left_cursor.saturating_sub(1), n);
    }
}

pub fn parse_layout_triplet(
    left_ref: &str,
    mid_ref: &str,
    right_ref: &str,
) -> Option<(u16, u16, u16)> {
    let l: u16 = left_ref.trim().parse().ok()?;
    let m: u16 = mid_ref.trim().parse().ok()?;
    let r: u16 = right_ref.trim().parse().ok()?;
    if l + m + r != 100 {
        return None;
    }
    Some((l, m, r))
}

/// Raw key for layout digit editing when unlocked and a layout field is focused.
pub fn handle_layout_text_key(state_mut: &mut UblxState, key: crossterm::event::KeyEvent) -> bool {
    if !state_mut.settings.layout_unlocked {
        return false;
    }
    let scope = state_mut.settings.scope;
    let btn = layout_button_index(scope);
    let cur = state_mut.settings.left_cursor;
    if cur <= btn || cur > btn + 3 {
        return false;
    }
    let field = cur - btn - 1;
    let buf = match field {
        0 => &mut state_mut.settings.layout_left_buf,
        1 => &mut state_mut.settings.layout_mid_buf,
        2 => &mut state_mut.settings.layout_right_buf,
        _ => return false,
    };
    match key.code {
        KeyCode::Char(c) if key.modifiers.intersection(KeyModifiers::CONTROL).is_empty() => {
            if c.is_ascii_digit() && buf.len() < 3 {
                buf.push(c);
            }
            true
        }
        KeyCode::Backspace => {
            buf.pop();
            true
        }
        _ => false,
    }
}
