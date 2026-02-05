# UBLX

[![Crates.io](https://img.shields.io/crates/v/ublx.svg)](https://crates.io/crates/ublx)
[![docs.rs](https://img.shields.io/docsrs/ublx)](https://docs.rs/ublx)
![Build](https://github.com/thicclatka/ublx/workflows/Build/badge.svg)
![Rust](https://img.shields.io/badge/rust-1.93-orange.svg)

[_Ublx ... Safe when taken as directed._](https://bookshop.org/p/books/ubik-philip-k-dick/1fc432e3ade32290)

UBLX is a **TUI that turns any directory into a flat, navigable catalog** — previews, metadata, and templates, all in the terminal. Not a generic file browser: give it a folder (research dump, project tree, backup) and you get one indexed view you can filter, search, and skim without drilling in and out of directories.

### What it can do now

- **Index a directory** — Walk the tree with [nefaxer](https://crates.io/crates/nefaxer) (drive-aware tuning, optional parallel walk). Uses a prior index when present (`.nefaxer` or existing `.ublx` snapshot) for faster diffs.
- **Enrich with metadata** — Run [zahirscan](https://crates.io/crates/zahirscan) on indexed paths (sequential or stream mode); results stored per path.
- **Single snapshot DB** — Writes `DIR/.ublx` (SQLite): snapshot table (path, mtime, size, hash, category, zahir JSON), settings (cached to skip disk check on next run), and delta_log (added/mod/removed). Config via `ublx.toml` or `.ublx.toml` in the directory.
- **Test run** — `ublx --test [DIR]` runs the full index + enrich pipeline without starting the TUI; logs duration at exit.
- **Minimal TUI** — Crossterm + Ratatui: notification bumper at bottom, optional dev log panel (`UBLX_DEV=1`). `q` / Esc to exit. (No snapshot list or 3-pane layout yet.)

### Goals (not yet implemented)

- **Navigable snapshot** — Browse the indexed tree in the TUI (list + categories).
- **Previews** — Peek at file contents (text, images, etc.) inline.
- **Templates** — Apply or generate from templates as you move through the tree.
- **Metadata pane** — See and filter by file type, size, dates, and other attributes.

## Usage

```bash
ublx [DIR]              # index DIR (default: current directory), then start TUI
ublx --test [DIR]       # index + enrich only, no TUI; logs duration
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
