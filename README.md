# UBLX

[![Crates.io](https://img.shields.io/crates/v/ublx.svg)](https://crates.io/crates/ublx)
[![docs.rs](https://img.shields.io/docsrs/ublx)](https://docs.rs/ublx)
![Build](https://github.com/thicclatka/ublx/workflows/Build/badge.svg)
![Rust](https://img.shields.io/badge/rust-1.93-orange.svg)

[_Ublx ... Safe when taken as directed._](https://bookshop.org/p/books/ubik-philip-k-dick/1fc432e3ade32290)

UBLX is a **TUI that turns any directory into a flat, navigable catalog** — previews, metadata, and templates in the terminal. Index once (nefaxer + zahirscan), then browse and search a single snapshot.

## Install

```bash
cargo install ublx
```

Or clone the repo and run `cargo build --release`; the binary is in `target/release/ublx`.

## What it does

- **Index & enrich** — [nefaxer](https://github.com/thicclatka/nefaxer) walks the tree (drive-aware); [zahirscan](https://github.com/thicclatka/zahirscan) adds metadata. Prior index (`.ublx` or `.nefaxer`) used for fast diffs. Writes `DIR/.ublx` (SQLite: snapshot, settings, delta_log, lenses). Config: `ublx.toml` or `.ublx.toml`.
- **TUI** — 3 panes: categories (left), contents (middle), right (Templates / Viewer / Metadata / Writing). Main tabs: **Snapshot** | **Delta** | **Lenses** (when present) | **Duplicates** (when present; Ctrl+d to run detection). Search (`/`), vim motions (j/k, h/l, gg/G), theme selector (Ctrl+t), context menus (Space, Shift+L), stacked toasts. Viewer has fullscreen (F). `q` / Esc quit.
- **Test run** — `ublx --test [DIR]` runs index + enrich only, no TUI.

## Modes (tabs)

| Tab            | Description                                                                                                                                                                       |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Snapshot**   | Categories (left) and file list (middle) from the current index; right pane shows Templates / Viewer / Metadata / Writing for the selected row.                                   |
| **Delta**      | Added / Mod / Removed since last snapshot; same 3-pane layout with overview in the right pane.                                                                                    |
| **Lenses**     | Saved lists of items with a specific focus (e.g. a “lens” on a subset of files); left = lens names, middle = paths in the selected lens. Shown when the DB has at least one lens. |
| **Duplicates** | Groups of duplicate files by content hash; left = group names, middle = paths in the group. Run duplicate detection (Ctrl+d) to populate; tab appears when groups exist.          |

Cycle tabs with **Shift+Tab** (Snapshot → Delta → Lenses → Duplicates → …).

## Panes overview

The right pane shows Viewer, Templates, Metadata, or Writing for the selected item. **Tab** switches focus between the left (categories) and middle (contents) panes; **h** / **l** also focus left or middle.

- **Focus a right-pane tab** — **v** (Viewer), **t** (Templates), **m** (Metadata), **w** (Writing). **Shift+V** cycles through right-pane tabs.
- **Scroll in the preview** — **Ctrl+b** / **Ctrl+e** (jump to top/bottom), **Shift+↑** / **Shift+↓** or **Shift+K** / **Shift+J** (line by line).
- **Viewer fullscreen** — **F** toggles fullscreen for the Viewer tab.
- **Search** — **/** filters the category and content lists (left and middle) by substring; the right pane updates with the selected row. Press **Esc** to clear the search.

**What each right-pane tab shows** (from [zahirscan](https://github.com/thicclatka/zahirscan) enrichment):

| Tab           | Content                                                                                                                                                                                                                                                                                                                       |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Viewer**    | Raw file content (text) or a type label for binaries; directory → `tree` output. Footer shows size and last-modified when available.                                                                                                                                                                                          |
| **Templates** | Extracted template/structure snippet (e.g. document outline) when zahirscan provides it.                                                                                                                                                                                                                                      |
| **Metadata**  | Enrichment metadata as **tables**: key/value pairs, and for supported types things like CSV column metadata, XLSX sheet stats (rows/columns per sheet), SQLite schema/table info, zip/archive “Contents” tables, and schema trees. Sections are parsed from the stored zahirscan result and rendered with headers and scroll. |
| **Writing**   | **Writing stats** (writing footprint): word count, character counts, and similar stats when zahirscan has computed them. Shown in the same table layout as Metadata.                                                                                                                                                          |

Press **?** in the TUI to open the full keybinding help.

## Configuration

Config is optional. If present, **global** config is applied first, then **local** overrides from the indexed directory.

| Platform      | Global config              |
| ------------- | -------------------------- |
| macOS / Linux | `~/.config/ublx/ublx.toml` |
| Windows       | `%APPDATA%\ublx\ublx.toml` |

**Local** config (same on all platforms): `.ublx.toml` or `ublx.toml` in the directory you index. Only keys present in each file override defaults (e.g. theme, layout pane percentages). Choosing a theme in the theme selector (Ctrl+t) and pressing Enter saves it to the local config.

**Live reload** — UBLX watches the config file. If you edit it (inside the TUI or in an external editor), a successful parse applies the new settings immediately. If the file is invalid, an error is shown and the last successful config is used; that snapshot is cached per indexed directory at:

| Platform      | Config cache                   |
| ------------- | ------------------------------ |
| macOS / Linux | `~/.local/share/ublx/configs/` |
| Windows       | `%LOCALAPPDATA%\ublx\configs\` |

## Usage

```bash
ublx [DIR]              # index DIR (default: .), then TUI
ublx --test [DIR]       # index + enrich only; logs duration
ublx --dev [DIR]        # dev mode: in-app log viewer, trace-level default; RUST_LOG overrides
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
