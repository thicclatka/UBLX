# KV tables

Rendering for the right-pane **Metadata** and **Writing** tabs: turn stored JSON (from zahirscan) into key/value and data tables with scroll.

Layout and scrollbar are handled by `render::scrollable_content`; here we parse JSON into sections and draw only the visible window (ratatui Table has no native scroll).

## Flow

1. **Input**: JSON string (metadata or writing_footprint) from `RightPaneContent`.
2. **Parse**: `sections::parse_json_sections` → list of `Section` (KeyValue, Contents, SingleColumnList).
3. **Draw**: `draw::draw_tables` slices to visible rows and renders tables in the given rect.

## Modules

| Module       | Purpose                                                                                                                                                          |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **sections** | Parse JSON blobs into `Section` (KeyValue, Contents, SingleColumnList). First section “General”; special keys: schema, sheet_stats, common_pivots, csv_metadata. |
| **walk**     | Map walk: root and nested objects → sections (flat KV, schema, sheet_stats, common_pivots, csv_metadata, entries).                                               |
| **csv**      | CSV metadata: column_types, column_names, etc. → tables.                                                                                                         |
| **xlsx**     | XLSX: sheet_stats (rows/columns per sheet) → table.                                                                                                              |
| **schema**   | Schema tree section from JSON.                                                                                                                                   |
| **format**   | Key/value and value display formatting.                                                                                                                          |
| **draw**     | `draw_tables(area, json, scroll_y)`: layout sections, slice to viewport, render.                                                                                 |
| **consts**   | Section key names, table gap.                                                                                                                                    |

Used by `render::panes::right` when `right_pane_mode` is Metadata or Writing.
