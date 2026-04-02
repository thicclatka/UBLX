# DB operations

SQLite `.ublx` database: schema, read/write, and higher-level ops (snapshot, delta, lenses, duplicates).

## Tables

| Table         | Purpose                                                                                                       |
| ------------- | ------------------------------------------------------------------------------------------------------------- |
| **snapshot**  | One row per path: path, mtime_ns, size, hash, category, zahir_json. Primary index.                            |
| **settings**  | Cached disk/tuning (num_threads, drive_type, parallel_walk, config_source) to skip disk check when DB exists. |
| **delta_log** | Per-run changes: created_ns, path, mtime_ns, size, hash, delta_type (added/mod/removed).                      |
| **lens**      | Lenses (id, name).                                                                                            |
| **lens_path** | Paths in each lens (lens_id, path).                                                                           |

Schema and DDL live in **consts** (`UblxDbSchema`, `create_ublx_db_sql()`).

## Modules

| Module           | Purpose                                                                                                        |
| ---------------- | -------------------------------------------------------------------------------------------------------------- |
| **consts**       | Schema, table/column names, SQL strings (create, insert, select). `DeltaType`, `UblxDbCategory`.               |
| **core**         | Open/create DB, write snapshot/settings/delta_log, load snapshot rows. `ensure_ublx_and_db`, `SnapshotTuiRow`. |
| **reader**       | Which DB to read from (`.ublx` vs `.ublx_tmp`) for live TUI during snapshot. `SnapshotReaderPreference`.       |
| **utils**        | Delta diff (copy_previous_delta_log, write delta rows), category helpers, cleanup.                             |
| **duplicates**   | Duplicate detection: group by hash or by size + blake3. `DuplicateGroup`, load for TUI.                        |
| **lens_storage** | Load lens names, load paths for a lens, create/rename/delete lens, add/remove path.                            |
