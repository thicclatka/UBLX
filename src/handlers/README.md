# Handlers

Handler logic for the TUI: input → state transitions, loading data, and calling into engine/render.

Indexer and Zahir integration (opts, batch/stream runs, delimiter helpers) live in **`crate::integrations`** (`nefax_ops`, `zahir_ops`) — see `src/integrations/`.

## Layout

| Module                | Purpose                                                                                                                                                                                                                                                                                                                 |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **core**              | `run_app` (headless snapshot, headless Zahir JSON export, or TUI) / `run_tui_session` (alternate screen + [`main_loop`](../app/mod.rs) + teardown). Startup snapshot respects `run_snapshot_on_startup` in config; config watcher on TUI path.                                                                          |
| **state_transitions** | Map key events to state changes (navigation, search, mode switch, open menu, lens menu, etc.).                                                                                                                                                                                                                          |
| **viewing**           | Right pane: [`core`](viewing/core.rs) (tree, file reader, `sectioned_preview_from_zahir`), [`zarrstore`](viewing/zarrstore.rs) (Zarr store = tree + zahir sections), [`async_ops`](viewing/async_ops.rs) (off-thread build for file rows and Zarr store dirs). `resolve_right_pane_content` / `drive_right_pane_async`. |
| **snapshot_pipeline** | Snapshot run (nefax + zahir, write to DB, toast). Used when user triggers “Take snapshot”.                                                                                                                                                                                                                              |
