# Handlers

Handler logic for the TUI: input → state transitions, loading data, and calling into engine/render.

Indexer and Zahir integration (opts, batch/stream runs, delimiter helpers) live in **`crate::integrations`** (`nefax_ops`, `zahir_ops`) — see `src/integrations/`.

## Layout

| Module                | Purpose                                                                                                                                            |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| **core**              | `run_app` (headless vs TUI pipeline) / `run_tui_session` (alternate screen + [`main_loop`](../app/mod.rs) + teardown). Config watcher on TUI path. |
| **state_transitions** | Map key events to state changes (navigation, search, mode switch, open menu, lens menu, etc.).                                                     |
| **viewing**           | Resolve right-pane content: file preview, tree for dirs, zahir JSON → templates/metadata/writing. `resolve_right_pane_content`.                    |
| **snapshot_pipeline** | Snapshot run (nefax + zahir, write to DB, toast). Used when user triggers “Take snapshot”.                                                         |
