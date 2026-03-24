# Handlers

Handler logic for the TUI: input → state transitions, loading data, and calling into engine/render.

## Layout

| Module                | Purpose                                                                                                                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **core**              | Top-level `run_ublx`: setup (DB, config, logging), run main loop, teardown. Owns the event loop entry.                          |
| **state_transitions** | Map key events to state changes (navigation, search, mode switch, open menu, lens menu, etc.).                                  |
| **viewing**           | Resolve right-pane content: file preview, tree for dirs, zahir JSON → templates/metadata/writing. `resolve_right_pane_content`. |
| **snapshot**          | Snapshot run (nefax + zahir, write to DB, toast). Used when user triggers “Take snapshot”.                                      |
| **applets**           | Small, named features with their own state/key handling.                                                                        |
| **wrappers**          | Thin wrappers around nefaxer and zahirscan (opts, run, result extraction).                                                      |

## Applets (`applets/`)

| Applet             | Purpose                                                                                |
| ------------------ | -------------------------------------------------------------------------------------- |
| **theme_selector** | Open selector (Ctrl+t), handle j/k/Enter/Esc, apply theme, toast.                      |
| **settings**       | First-tick toast, config watcher for hot reload.                                       |
| **first_run**      | First-run prompt to choose default `enable_enhance_all` and write local config.         |
| **enhance_policy** | Space-menu flow to set per-subtree `[[enhance_policy]]` (auto vs manual batch Zahir). |
| **enhance**        | Per-file “Enhance with ZahirScan” when global enhance is off.                          |
| **dupe_finder**    | Spawn duplicate detection in background; on result, toast or switch to Duplicates tab. |
| **opener**         | Open (Terminal) / Open (GUI) from context menu.                                        |
| **lens**           | Add to lens, create/rename/delete lens; lens menu and confirm flows.                   |

## Wrappers (`wrappers/`)

| Wrapper       | Purpose                                                                             |
| ------------- | ----------------------------------------------------------------------------------- |
| **nefax_ops** | Build nefax opts, run nefax (batch or stream), map results.                         |
| **zahir_ops** | Build zahir config from ublx opts, run zahir (batch or stream), empty-path short circuit, get output by path. |
