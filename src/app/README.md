# App (TUI loop)

Main TUI loop and view-data construction. The loop runs in [`main_loop`](runtime/mod.rs); setup/teardown live in [`handlers::core::run_tui_session`](../handlers/core.rs).

## Modules

| Module            | Purpose                                                                                                                                  |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| **runtime**       | `main_loop` (`mod.rs`). One tick: `tick.rs` (applets, snapshot, view build, draw, input), `frame.rs` (`DrawFrameArgs`, draw), `view_build.rs` (`build_view_and_right_content`). |
| **params**        | `RunUblxParams`: DB path, layout, bumper, lens names, duplicate load channel, etc. Passed into the loop.                                 |
| **view_data**     | Snapshot-mode view: filter categories/contents by search, clamp selection, build `ViewData`. Shared helpers for delta and user-selected. |
| **snapshot**      | `load_snapshot_for_tui`: load categories and rows from DB for the TUI (Snapshot mode). Reader preference for live vs stable DB.          |
| **delta**         | Delta-mode view: load delta_log by type (added/mod/removed), build `DeltaViewData`.                                                      |
| **user_selected** | Duplicates and Lenses modes: build `ViewData` from duplicate groups or lens paths; shared two-pane structure.                            |

## View data flow

- **Snapshot**: `load_snapshot_for_tui` → full rows; `view_data` filters by category + search, builds `ViewData` with `SnapshotIndices`.
- **Delta**: `delta` loads `DeltaViewData`; `main_loop` passes it to render for delta panes.
- **Duplicates / Lenses**: `user_selected` builds `ViewData` from `UserSelectedSource` (groups or lens paths); same pane layout as Snapshot, different data source.

Selection clamping and preview-scroll reset happen in `view_data` when category or content selection changes.
