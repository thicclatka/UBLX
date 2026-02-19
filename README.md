# UBLX

[![Crates.io](https://img.shields.io/crates/v/ublx.svg)](https://crates.io/crates/ublx)
[![docs.rs](https://img.shields.io/docsrs/ublx)](https://docs.rs/ublx)
![Build](https://github.com/thicclatka/ublx/workflows/Build/badge.svg)
![Rust](https://img.shields.io/badge/rust-1.93-orange.svg)

[_Ublx ... Safe when taken as directed._](https://bookshop.org/p/books/ubik-philip-k-dick/1fc432e3ade32290)

UBLX is a **TUI that turns any directory into a flat, navigable catalog** — previews, metadata, and templates in the terminal. Index once (nefaxer + zahirscan), then browse and search a single snapshot.

## What it does

- **Index & enrich** — [nefaxer](https://github.com/thicclatka/nefaxer) walks the tree (drive-aware); [zahirscan](https://github.com/thicclatka/zahirscan) adds metadata. Prior index (`.ublx` or `.nefaxer`) used for fast diffs. Writes `DIR/.ublx` (SQLite: snapshot, settings, delta_log). Config: `ublx.toml` or `.ublx.toml`.
- **TUI** — 3 panes: categories (left), contents (middle), right (Templates / Viewer / Metadata / Writing). Tabs: Snapshot | Delta. Search (`/`), vim motions (j/k, h/l, gg/G), theme selector (Shift+T), stacked toasts. Viewer has fullscreen (F). `q` / Esc quit.
- **Test run** — `ublx --test [DIR]` runs index + enrich only, no TUI.

## Usage

```bash
ublx [DIR]              # index DIR (default: .), then TUI
ublx --test [DIR]       # index + enrich only; logs duration
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
