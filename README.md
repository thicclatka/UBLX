# UBLX

[![Crates.io](https://img.shields.io/crates/v/ublx.svg)](https://crates.io/crates/ublx)
[![docs.rs](https://img.shields.io/docsrs/ublx)](https://docs.rs/ublx)
![Build](https://github.com/thicclatka/ublx/workflows/Build/badge.svg)
![Rust](https://img.shields.io/badge/rust-1.93-orange.svg)

[_Ublx ... Safe when taken as directed._](https://bookshop.org/p/books/ubik-philip-k-dick/1fc432e3ade32290)

UBLX is a **TUI that turns any directory into a flat, navigable catalog** — index once, then browse categories, previews, metadata, and templates in the terminal. (Driven by [nefaxer](https://github.com/thicclatka/nefaxer) and [zahirscan](https://github.com/thicclatka/zahirscan))

**_Currently in development, expect breaking changes._**

## Install

```bash
cargo install ublx

# or clone repo
cargo build --release
```

## What it does

- **Index once, then browse** — One run gives you a flat catalog with categories, file list, previews, metadata tables, and templates. Prior index is used for fast diffs. Writes a per-root SQLite file under your user cache (`ubli/`; stem is sanitized dir name plus path hash, extension matches the `ublx` package name). Config: `ublx.toml` or `.ublx.toml`.
- **TUI** — 3 panes: categories (left), contents (middle), file viewer (right). Main tabs: **Snapshot** | **Delta** | **Lenses** | **Duplicates** | **Settings**. Search across & within files, vim motions, theme selector, quick actions menu, command mode, toast notifications, fullscreen toggle.
- **Snapshot-only** — Headless index without the TUI
- **Export** — Headless `--export` / `-x` writes pretty-printed Zahir JSON from the snapshot DB into `ublx-export/` as flat `{path}.json` files

## Use case

Looking for a file manager? Use [yazi](https://github.com/sxyazi/yazi) for that; where UBLX comes in:

- Best for navigating and handling project directories that you frequent
- Preview & extract filetype specific metadata without having to open a file
- **For large directories**: can still get a quick understanding of filetypes contained without metadata enhancement

## Modes

| Tab            | Description                                                                                                                                                                       |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Snapshot**   | Categories (left) and file list (middle) from the current index; right pane shows Templates / Viewer / Metadata / Writing for the selected row.                                   |
| **Delta**      | Added / Mod / Removed since last snapshot; same 3-pane layout with overview in the right pane.                                                                                    |
| **Lenses**     | Saved lists of items with a specific focus (e.g. a “lens” on a subset of files); left = lens names, middle = paths in the selected lens. Shown when the DB has at least one lens. |
| **Duplicates** | Groups of duplicate files by content hash; left = group names, middle = paths in the group. Run duplicate detection (Ctrl+d) to populate; tab appears when groups exist.          |
| **Settings**   | Global vs local config for the indexed root: theme, layout, `bg_opacity`, and other hot-reloadable keys; **e** opens the active scope's file in `$EDITOR`.                        |

Cycle main tabs with `~`.

## Panes overview

The right pane shows Viewer, Templates, Metadata, or Writing for the selected item. **Tab** switches focus between the left (categories) and middle (contents) panes; **h** / **l** also focus left or middle.

- **Focus a right-pane tab** — **v** (Viewer), **t** (Templates), **m** (Metadata), **w** (Writing). **Shift+Tab** cycles through right-pane tabs.
- **Scroll in the preview** — **Shift+b** / **Shift+e** (jump to top/bottom), **Shift+↑** / **Shift+↓** or **Shift+K** / **Shift+J** (line by line).
- **Viewer fullscreen** — **Shift+F** toggles fullscreen for the Viewer tab.
- **Viewer search** — **Shift+S** opens literal in-pane search in the preview (see in-app help for n/N and Esc).
- **Catalog search** — **/** filters the category and content lists (left and middle) by substring; the right pane updates with the selected row. Press **Esc** to clear filter.

| Tab           | Content                                                                                                                                                                                                                                                                                                                       |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Viewer**    | Previews for the selected file — details in [Viewer](#viewer). Footer: size and last-modified when available.                                                                                                                                                                                                                 |
| **Templates** | Extracted template/structure snippet (e.g. document outline) when zahirscan provides it.                                                                                                                                                                                                                                      |
| **Metadata**  | Enrichment metadata as **tables**: key/value pairs, and for supported types things like CSV column metadata, XLSX sheet stats (rows/columns per sheet), SQLite schema/table info, zip/archive “Contents” tables, and schema trees. Sections are parsed from the stored zahirscan result and rendered with headers and scroll. |
| **Writing**   | **Writing stats** (writing footprint): word count, character counts, and similar stats when zahirscan has computed them. Shown in the same table layout as Metadata.                                                                                                                                                          |

### Viewer

- **Markdown** — formatted preview (headings, lists, code blocks, tables inside the doc).
- **CSV-style files** — pretty table layout for `.csv`, `.tsv`, `.tab`, `.psv` when the index says so or the path matches (so previews still work if a row’s category label is off).
- **Images** — terminal preview via [ratatui-image](https://github.com/ratatui-org/ratatui-image) (downscaled for the pane; larger files may decode off the UI thread; recent previews cached for quick navigation).
- **Code and structured text** — [syntect](https://github.com/trishume/syntect) highlighting via [`sublime_syntaxes`](https://crates.io/crates/sublime_syntaxes); grammar from path/extension; colors match theme light/dark. Large buffers are cached for smooth scrolling.
- **Other text** — raw text (length-capped).
- **Binaries** — short label instead of dumping bytes.
- **Directories** — `tree` when available.

Press **?** to open the full keybinding help.

## Configuration

Config is optional. If present, **global** config is applied first, then **local** overrides from the indexed directory. Successful configs are cached per indexed directory.

| Platform      | Global config              | Config Cache                   |
| ------------- | -------------------------- | ------------------------------ |
| macOS / Linux | `~/.config/ublx/ublx.toml` | `~/.local/share/ublx/configs/` |
| Windows       | `%APPDATA%\ublx\ublx.toml` | `%LOCALAPPDATA%\ublx\configs\` |

**Local** config (same on all platforms): `.ublx.toml` (default) or `ublx.toml` in the directory you index. Only keys present in each file override defaults. Choosing a theme in the theme selector (Ctrl+t) and confirming choice with Enter saves it to the local config.

**Configurable keys**:

| Key                  | Type                 | Allowable values / notes                                                                                                                                                                         |
| -------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `theme`              | string               | See [Themes](src/themes/README.md#allowable-values).                                                                                                                                             |
| `layout`             | table                | Pane widths: `left_pct`, `middle_pct`, `right_pct` (each 0–100; must sum to 100). Default: `left_pct = 10`, `middle_pct = 30`, `right_pct = 60`.                                                 |
| `bg_opacity`         | float (optional)     | Page background opacity `0.0`–`1.0`: main pane uses terminal default fill (OSC 11) below `1.0` so wallpaper can show through; omitted or `1.0` = solid theme background. Adjustable in Settings. |
| `opacity_format`     | string (optional)    | When `bg_opacity` &lt; 1: OSC 11 payload style — `rgba` (default) or `hex8` (`#RRGGBBAA`). Some terminals prefer one or the other.                                                               |
| `show_hidden_files`  | bool                 | If `true`, include hidden files (e.g. `.*`) in the index.                                                                                                                                        |
| `hash`               | bool                 | If `true`, compute blake3 hash per file (slower; used for duplicate detection and change detection).                                                                                             |
| `exclude`            | array of strings     | Extra path patterns to exclude from indexing (startup only; not hot-reloadable).                                                                                                                 |
| `editor_path`        | string               | Path to editor for “Open (Terminal)” (e.g. `"vim"`, `"nvim"`). When unset, uses `$EDITOR`.                                                                                                       |
| `enable_enhance_all` | bool                 | If `true`, metadata for all files on snapshot. If `false` (default), path-only until **Enhance with ZahirScan** per file.                                                                        |
| `[[enhance_policy]]` | TOML array of tables | Optional per-subtree rules (see below). Hot-reloadable with the rest of the overlay.                                                                                                             |

All of the above except **`exclude`** are **hot-reloadable**.

**`[[enhance_policy]]`** — Each row has `path` (relative to the indexed directory, `/` separators, e.g. `src` or `photos/2024`) and `policy`:

- **`auto`** — ZahirScan runs for files under that prefix when you take a snapshot (same idea as `enable_enhance_all = true`, but only for this subtree).
- **`manual`** — No batch Zahir on snapshot for that subtree (same idea as `enable_enhance_all = false`): the catalog is path-only there until you **Enhance with ZahirScan** on specific files.

The **longest** `path` prefix that matches a file wins. If no row matches, **`enable_enhance_all`** applies globally.

**Live reload** — UBLX watches the config file. If you edit it (inside the TUI or in an external editor), a successful parse applies the new settings immediately; valid merged overlay is written to the per-directory config cache. If the file is invalid, an error is shown and the last successful config saved in cache is used.

## Usage

```text
Usage: ublx [OPTIONS] <DIR>

Arguments:
  <DIR>  Directory to index (default: current directory)

Options:
  -s, --snapshot-only  Headless snapshot. Writes a local config file when this dir has none
  -e, --enhance-all    With `--snapshot-only`: set `enable_enhance_all = true` in new local config and use it for this run
  -f, --full-snapshot  Same as `--snapshot-only --enhance-all`
  -x, --export         Headless: write each Zahir JSON to `ublx-export/` as flat `{path}.json` files. Recommended to run with "--full-snapshot" to get most complete & recent results. Adjust enhance policy in config to fine-tune which paths get ZahirScan
      --dev            Dev mode: tui-logger drain + `move_events` + trace-level default filter
      --themes         Print available themes grouped by appearance
  -h, --help           Print help
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
