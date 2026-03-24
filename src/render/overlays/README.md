# Overlays

All overlay **drawing** (full-screen or floating UI above main content) lives in this crate. Layout holds state and theme data; render owns `Frame` drawing for overlays.

## Modules

| Module             | Purpose                                                                                                                            |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| **help**           | Keybinding help box (`?`). `render_help_box`.                                                                                      |
| **theme_selector** | Theme picker (Ctrl+t). `render_theme_selector`.                                                                                    |
| **popup**          | Context menus and list popups: open menu, lens menu, space menu, enhance policy, first-run prompt, delete confirm; shared list/text-input utils. |
| **toast**          | Stacked toast notifications. `render_toast_slot`. Toast _data_ (slots, bumper, `show_toast_slot`) lives in `utils::notifications`. |

Entry from `render::core`: overlays are drawn after main content (help, theme selector, then popups, then toasts).
