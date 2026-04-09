# Modules

Small, named features used from handlers and UI: their own state and/or key handling, callable via `crate::modules`.

| Module             | Purpose                                                                                                                             |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **theme_selector** | Open selector (Command Mode: **Ctrl+A**, then **t**), handle j/k/Enter/Esc, apply theme, toast.                                     |
| **settings**       | First-tick toast, config watcher for hot reload (`settings/`); bool rows edit merged overlay keys (e.g. `run_snapshot_on_startup`). |
| **first_run**      | First-run prompt to choose default `enable_enhance_all` and write local config.                                                     |
| **enhance_policy** | Quick actions (spacebar) flow to set per-subtree `[[enhance_policy]]` (auto vs manual batch Zahir).                                 |
| **enhance**        | Per-file “Enhance with ZahirScan” when global enhance is off.                                                                       |
| **dupe_finder**    | Spawn duplicate detection in background; on result, toast or switch to Duplicates tab.                                              |
| **opener**         | Open (Terminal) / Open (GUI) from context menu.                                                                                     |
| **lenses**         | Add to lens, create/rename/delete lens; lens menu and confirm flows.                                                                |
| **catalog_filter** | Fuzzy catalog search over paths/categories (pure helpers; callers update `ViewData`).                                               |
| **ublx_switch**    | Switch indexed project: recents-backed roots, in-process root change.                                                               |
| **viewer_search**  | Literal in-pane viewer search (Shift+S): ranges, scroll, highlight.                                                                 |
| **exporter**       | Background Zahir JSON (`x`) and lens Markdown (`l`) exports; worker thread + toasts.                                                |
| **file_ops**       | Rename/delete under indexed root (quick actions menu (spacebar)); DB and lens path updates.                                         |
