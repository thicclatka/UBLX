/// Builds a panel block title: `" Label "` or `" ► Label "` when focused.
pub fn panel_title(label: &str, focused: bool) -> String {
    if focused {
        format!(" ► {} ", label)
    } else {
        format!(" {} ", label)
    }
}

/// All symbols and string literals used by the renderer. Single place to tweak UI copy/symbols.
pub struct UiStrings {
    pub categories: &'static str,
    pub contents: &'static str,
    // Right pane block titles
    pub viewer: &'static str,
    pub templates: &'static str,
    pub metadata: &'static str,
    pub writing: &'static str,
    pub search_title: &'static str,
    // Tab bar labels and separator
    pub tab_templates: &'static str,
    pub tab_viewer: &'static str,
    pub tab_metadata: &'static str,
    pub tab_writing: &'static str,
    pub tab_sep: &'static str,
    // List panel
    pub all_categories: &'static str,
    pub no_contents: &'static str,
    pub no_matches: &'static str,
    pub list_highlight: &'static str,
    pub list_unfocused: &'static str,
    /// Search and hints
    pub search_clear_hint_prefix: &'static str,
    pub search_prompt: &'static str,
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
}

impl UiStrings {
    pub const fn new() -> Self {
        Self {
            categories: "Categories",
            contents: "Contents",
            viewer: " Viewer ",
            templates: " Templates ",
            metadata: " Metadata ",
            writing: " Writing ",
            search_title: " Search ",
            tab_templates: "Templates",
            tab_viewer: "Viewer",
            tab_metadata: "Metadata",
            tab_writing: "Writing",
            tab_sep: " | ",
            all_categories: "All",
            no_contents: "(no contents)",
            no_matches: "(no matches)",
            list_highlight: "▌ ",
            list_unfocused: "  ",
            search_clear_hint_prefix: " Esc to clear (current query: ",
            search_prompt: " / ",
            not_available: "(not available for this item)",
            viewer_placeholder: "(viewer — file content will load here)",
            main_tab_snapshot: "Snapshot",
            main_tab_delta: "Delta",
            delta_added: "Added",
            delta_mod: "Modified",
            delta_removed: "Removed",
            delta_right_title: " Snapshot overview ",
        }
    }
}
