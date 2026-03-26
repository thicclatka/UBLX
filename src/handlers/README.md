# Handlers

Handler logic for the TUI: input → state transitions, loading data, and calling into engine/render.

Indexer and Zahir integration (opts, batch/stream runs, delimiter helpers) live in **`crate::integrations`** (`nefax_ops`, `zahir_ops`) — see `src/integrations/`.

## Layout

| Module                | Purpose                                                                                                                                        |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **core**              | `run_app` / `run_ublx`: test vs TUI, terminal setup/teardown, config watcher. TUI path calls [`main_loop`](../app/mod.rs) after setup. |
| **state_transitions** | Map key events to state changes (navigation, search, mode switch, open menu, lens menu, etc.).                                                 |
| **viewing**           | Resolve right-pane content: file preview, tree for dirs, zahir JSON → templates/metadata/writing. `resolve_right_pane_content`.                |
| **snapshot**          | Snapshot run (nefax + zahir, write to DB, toast). Used when user triggers “Take snapshot”.                                                     |
| **applets**           | Small, named features with their own state/key handling.                                                                                       |

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
