//! Command Mode (`Ctrl+A`) action descriptions (shared with help + chord menu).

/// Second-column descriptions for Command Mode + d/t/s/r/p (same strings as the help overlay’s Command Mode rows).
#[derive(Clone, Copy, Debug)]
pub struct CommandModeDescriptions {
    pub duplicates: &'static str,
    pub theme: &'static str,
    pub snapshot: &'static str,
    pub reload: &'static str,
    pub export_zahir: &'static str,
    pub export_lenses: &'static str,
    pub project: &'static str,
}

pub const COMMAND_MODE_DESCRIPTIONS: CommandModeDescriptions = CommandModeDescriptions {
    duplicates: "Run duplicate detection",
    theme: "Theme selector",
    snapshot: "Take snapshot",
    reload: "Reload config from disk",
    export_zahir: "Export Zahir JSON (ublx-export/)",
    export_lenses: "Export lenses as Markdown (ublx-lenses/)",
    project: "Switch UBLX project",
};
