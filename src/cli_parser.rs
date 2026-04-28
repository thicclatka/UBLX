use std::path::PathBuf;

use clap::Parser;
use log::debug;

use crate::themes;
use crate::utils;

#[derive(Parser)]
#[command(
    name = "ublx",
    version,
    about = "UBLX is a TUI to index once, enrich with metadata, and browse a flat snapshot in a 3-pane layout with multiple modes."
)]
pub struct Args {
    /// Directory to index
    #[arg(value_name = "DIR", default_value = ".")]
    pub dir_to_ublx: PathBuf,
    #[command(flatten)]
    pub headless: HeadlessCli,
    /// Dev mode: tui-logger drain + `move_events` + trace-level default filter
    #[arg(long = "dev")]
    pub dev: bool,
    /// Print available themes grouped by appearance
    #[arg(long = "themes")]
    pub themes: bool,
}

/// Headless indexing flag
#[derive(Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct HeadlessCli {
    /// Headless snapshot. Writes a local config file when this dir has none.
    #[arg(long = "snapshot-only", short = 's')]
    pub snapshot_only: bool,
    /// With `--snapshot-only`: set `enable_enhance_all = true` in new local config and use it for this run.
    #[arg(long = "enhance-all", short = 'e')]
    pub enhance_all: bool,
    /// Same as `--snapshot-only --enhance-all`.
    #[arg(long = "full-snapshot", short = 'f')]
    pub full_snapshot: bool,
    /// Headless: write each Zahir JSON to `ublx-export/` as flat `{path}.json` files. Recommended to run with "--full-snapshot" to get most complete & recent results. Adjust enhance policy in config to fine-tune which paths get `ZahirScan`.
    #[arg(long = "export", short = 'x')]
    pub export_zahir: bool,
}

pub fn print_available_themes() {
    for entry in themes::theme_selector_entries() {
        match entry {
            themes::SelectorEntry::Section(label) => {
                println!("{label}:");
            }
            themes::SelectorEntry::Item(theme) => {
                println!("  - {}", theme.name);
            }
        }
    }
}

/// Headless snapshot flavor: `-s` (optionally with `-e`) or `-f` (implies enhance-all for the run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotType {
    /// `--snapshot-only`; `enhance_all` reflects `--enhance-all`.
    MinSnapshot { enhance_all: bool },
    /// `--full-snapshot` (same as `--snapshot-only --enhance-all` for this run).
    FullSnapshot,
}

/// Normalized headless CLI: optional snapshot pass, optional export pass (both may be set → snapshot then export).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessModeFlags {
    /// `None` when neither `-s` nor `-f`.
    pub snapshot: Option<SnapshotType>,
    /// `--export` / `-x`.
    pub export: bool,
}

impl HeadlessModeFlags {
    #[must_use]
    pub fn new(args_headless: &HeadlessCli) -> Self {
        let snapshot = if args_headless.full_snapshot {
            Some(SnapshotType::FullSnapshot)
        } else if args_headless.snapshot_only {
            Some(SnapshotType::MinSnapshot {
                enhance_all: args_headless.enhance_all,
            })
        } else {
            None
        };
        Self {
            snapshot,
            export: args_headless.export_zahir,
        }
    }

    /// True when any headless work runs (no TUI): snapshot pass and/or export.
    #[must_use]
    pub fn is_headless(self) -> bool {
        self.snapshot.is_some() || self.export
    }

    /// Whether this headless snapshot run should enable enhance-all (`-f`, or `-s -e`).
    #[must_use]
    pub fn determine_enhance_all(self) -> bool {
        match self.snapshot {
            Some(SnapshotType::FullSnapshot) => true,
            Some(SnapshotType::MinSnapshot { enhance_all }) => enhance_all,
            None => false,
        }
    }
}

#[must_use]
pub fn headless_handler(args_headless: &HeadlessCli) -> HeadlessModeFlags {
    let flags = HeadlessModeFlags::new(args_headless);
    if args_headless.full_snapshot && args_headless.enhance_all {
        debug!("Full snapshot with --enhance-all is redundant; use --full-snapshot (-f) alone.");
    }
    utils::exit_if_enhance_all_without_headless(
        args_headless.enhance_all,
        flags.snapshot.is_some(),
    );
    flags
}
