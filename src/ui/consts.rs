/// All symbols and string literals used by the renderer. Single place to tweak UI copy/symbols.
pub struct UiStrings {
    pub categories: &'static str,
    pub contents: &'static str,
    // Right pane block titles
    pub viewer: &'static str,
    pub templates: &'static str,
    pub metadata: &'static str,
    pub writing: &'static str,
    // Tab bar labels
    pub tab_templates: &'static str,
    pub tab_viewer: &'static str,
    pub tab_metadata: &'static str,
    pub tab_writing: &'static str,
    // List panel
    pub all_categories: &'static str,
    pub no_contents: &'static str,
    pub no_matches: &'static str,
    /// Symbol before each list row (selection shown by title + highlight style).
    pub list_symbol: &'static str,
    /// Status line: label before search query (e.g. "Search: ").
    pub status_search_label: &'static str,
    /// Status line: "Esc to clear" label next to search.
    pub status_esc_to_clear: &'static str,
    /// Right pane placeholders
    pub not_available: &'static str,
    pub viewer_placeholder: &'static str,
    /// Main mode tabs
    pub main_tab_snapshot: &'static str,
    pub main_tab_delta: &'static str,
    /// Delta left-pane labels
    pub delta_added: &'static str,
    pub delta_mod: &'static str,
    pub delta_removed: &'static str,
    pub delta_right_title: &'static str,
    /// UBLX Settings config labels
    pub global_config: &'static str,
    pub local_config: &'static str,
    // Status / delta / popups (raw; use [Self::pad] for block titles)
    pub latest_snapshot_label: &'static str,
    pub delta_block_title: &'static str,
    pub delta_loading: &'static str,
    pub delta_placeholder_dash: &'static str,
    pub delta_type_label: &'static str,
    pub paths_label: &'static str,
    pub brand: &'static str,
    pub fullscreen_suffix: &'static str,
    pub table_header_key: &'static str,
    pub table_header_value: &'static str,
    pub help_title: &'static str,
    pub theme_title: &'static str,
    pub notification_title: &'static str,
    pub first_table_title: &'static str,
    pub contents_table_title: &'static str,
    pub help_table_command: &'static str,
    pub help_table_action: &'static str,
}

impl UiStrings {
    /// Pads a label with spaces for block/popup titles, e.g. `pad("Delta")` → `" Delta "`.
    #[must_use]
    pub fn pad(&self, s: &str) -> String {
        format!(" {} ", s)
    }

    pub const fn new() -> Self {
        Self {
            categories: "Categories",
            contents: "Contents",
            viewer: "Viewer",
            templates: "Templates",
            metadata: "Metadata",
            writing: "Writing",
            tab_templates: "Templates",
            tab_viewer: "Viewer",
            tab_metadata: "Metadata",
            tab_writing: "Writing",
            all_categories: "All",
            no_contents: "(no contents)",
            no_matches: "(no matches)",
            list_symbol: "  ",
            status_search_label: "Search: ",
            status_esc_to_clear: "Esc to clear",
            not_available: "(not available for this item)",
            viewer_placeholder: "(viewer — file content will load here)",
            main_tab_snapshot: "Snapshot",
            main_tab_delta: "Delta",
            delta_added: "Added",
            delta_mod: "Modified",
            delta_removed: "Removed",
            delta_right_title: "Snapshot overview",
            global_config: "Global",
            local_config: "Local",
            latest_snapshot_label: "Latest Snapshot",
            delta_block_title: "Delta",
            delta_loading: "Loading…",
            delta_placeholder_dash: "—",
            delta_type_label: "Delta type",
            paths_label: "Paths",
            brand: "UBLX",
            fullscreen_suffix: "(Esc to exit fullscreen)",
            table_header_key: "Key",
            table_header_value: "Value",
            help_title: "Help",
            theme_title: "Theme",
            notification_title: "Notification",
            first_table_title: "General",
            contents_table_title: "Contents",
            help_table_command: "Command",
            help_table_action: "Action",
        }
    }
}

pub const UI_STRINGS: UiStrings = UiStrings::new();

/// Shared UI layout constants (padding, etc.).
pub struct UiConstants {
    pub h_pad: u16,
    pub v_pad: u16,
    pub table_stripe_lighten: f32,
}

impl UiConstants {
    pub const fn new() -> Self {
        Self {
            h_pad: 1,
            v_pad: 1,
            table_stripe_lighten: 0.06,
        }
    }
}

pub const UI_CONSTANTS: UiConstants = UiConstants::new();

/// Tree-drawing characters for directory-style trees (e.g. schema tree, file tree). Single place to tweak box-drawing.
pub struct TreeChars {
    /// Non-last sibling: "├─ "
    pub branch: &'static str,
    /// Last sibling: "└─ "
    pub last_branch: &'static str,
    /// Continuation (more siblings below): "│  "
    pub vertical: &'static str,
    /// No continuation (last branch): "   "
    pub space: &'static str,
}

impl TreeChars {
    pub const fn new() -> Self {
        Self {
            branch: "├─ ",
            last_branch: "└─ ",
            vertical: "│  ",
            space: "   ",
        }
    }
}

pub const TREE_CHARS: TreeChars = TreeChars::new();
