use ratatui::layout::Constraint;
use ratatui::style::Style;
use ratatui::text::Span;

use crate::layout::setup::MainMode;
use crate::utils::StringObjTraits;

/// Generic and feature-specific loading lines.
pub struct UiStringsLoading {
    /// Short spinner / placeholder (e.g. delta pane while data loads).
    pub general: &'static str,
}

/// Delta mode: section titles and row labels.
pub struct UiStringsDelta {
    pub added: &'static str,
    pub modified: &'static str,
    pub removed: &'static str,
    pub right_title: &'static str,
    /// Left pane block title (delta list).
    pub left_block_title: &'static str,
    pub placeholder_dash: &'static str,
    pub type_label: &'static str,
}

/// Snapshot / viewer pane titles and tab labels.
pub struct UiStringsPane {
    pub categories: &'static str,
    pub contents: &'static str,
    pub viewer: &'static str,
    pub templates: &'static str,
    pub metadata: &'static str,
    pub writing: &'static str,
    pub tab_templates: &'static str,
    pub tab_viewer: &'static str,
    pub tab_metadata: &'static str,
    pub tab_writing: &'static str,
    pub not_available: &'static str,
    pub viewer_placeholder: &'static str,
}

/// Middle / list column (All, empty states, row prefix).
pub struct UiStringsList {
    pub all_categories: &'static str,
    pub no_contents: &'static str,
    pub no_matches: &'static str,
    pub list_symbol: &'static str,
}

/// Main mode tab bar: Snapshot | Delta | …
pub struct UiStringsMainTabs {
    pub snapshot: &'static str,
    pub delta: &'static str,
    pub settings: &'static str,
    pub duplicates: &'static str,
    pub lenses: &'static str,
}

/// Digit keys (1–9) for main-mode tabs. Single source for keymap, tab labels, mouse hit-testing, and help.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UblxTabNumber {
    pub snapshot: u8,
    pub delta: u8,
    pub settings: u8,
    pub lenses: u8,
    pub duplicates: u8,
}

impl UblxTabNumber {
    /// Snapshot [1], Lenses [2], Delta [7], Duplicates [8], Settings [9] — matches left-to-right tab bar order.
    pub const DEFAULT: Self = Self {
        snapshot: 1,
        lenses: 2,
        delta: 7,
        duplicates: 8,
        settings: 9,
    };
}

pub const MAIN_TAB_KEYS: UblxTabNumber = UblxTabNumber::DEFAULT;

/// Tab label with hotkey digit, e.g. `Settings [9]` — use with [`MAIN_TAB_KEYS`] and [`UiStringsMainTabs`].
#[must_use]
pub fn main_tab_title(label: &str, key_digit: u8) -> String {
    format!("{label} [{key_digit}]")
}

/// Main tab bar order and labels (Snapshot, optional Lenses, Delta, optional Duplicates, Settings).
/// Matches [`crate::render::core::draw_main_tabs`] segment order and mouse hit-testing.
#[must_use]
pub fn main_tab_bar_modes_and_labels(
    has_lenses: bool,
    has_duplicates: bool,
) -> (Vec<MainMode>, Vec<String>) {
    let k = MAIN_TAB_KEYS;
    let mut modes = vec![MainMode::Snapshot];
    let mut labels = vec![main_tab_title(UI_STRINGS.main_tabs.snapshot, k.snapshot)];
    if has_lenses {
        modes.push(MainMode::Lenses);
        labels.push(main_tab_title(UI_STRINGS.main_tabs.lenses, k.lenses));
    }
    modes.push(MainMode::Delta);
    labels.push(main_tab_title(UI_STRINGS.main_tabs.delta, k.delta));
    if has_duplicates {
        modes.push(MainMode::Duplicates);
        labels.push(main_tab_title(
            UI_STRINGS.main_tabs.duplicates,
            k.duplicates,
        ));
    }
    modes.push(MainMode::Settings);
    labels.push(main_tab_title(UI_STRINGS.main_tabs.settings, k.settings));
    (modes, labels)
}

/// Help header: digits in tab-bar order (Snapshot, Lenses, Delta, Duplicates, Settings).
#[must_use]
pub fn main_tab_keys_help_keys_line() -> String {
    let k = MAIN_TAB_KEYS;
    format!(
        "{} | {} | {} | {} | {}",
        k.snapshot, k.lenses, k.delta, k.duplicates, k.settings
    )
}

/// Status / search line (snapshot + query).
pub struct UiStringsSearchStatus {
    pub search_label: &'static str,
    pub find_label: &'static str,
    pub latest_snapshot: &'static str,
}

/// UBLX settings source labels (global vs local config).
pub struct UiStringsConfig {
    pub global: &'static str,
    pub local: &'static str,
}

/// Paths column and group labels (duplicates / lenses).
pub struct UiStringsPaths {
    pub paths: &'static str,
    pub duplicate_group: &'static str,
    pub lens_group: &'static str,
}

pub struct UiStringsBrand {
    pub brand: &'static str,
    pub fullscreen_suffix: &'static str,
}

pub struct UiStringsTables {
    pub header_key: &'static str,
    pub header_value: &'static str,
    pub first_title: &'static str,
    pub contents_title: &'static str,
    pub columns_title: &'static str,
}

/// Modal / overlay titles and table column headers for help.
pub struct UiStringsDialogs {
    pub help: &'static str,
    pub theme: &'static str,
    pub notification: &'static str,
    pub help_command: &'static str,
    pub help_action: &'static str,
}

/// Image / PDF / raster preview chrome and error prefixes (detail after `: ` from `format!`).
pub struct UiStringsViewerRaster {
    /// PDF footer: `format!("{} {p} / {n}", self.page_label, ...)`.
    pub page_label: &'static str,
    pub could_not_load_preview: &'static str,
    pub could_not_decode_cover: &'static str,
    pub could_not_open_image: &'static str,
}

pub struct UiStringsToasts {
    pub config_reloaded: &'static str,
    pub no_duplicates: &'static str,
    /// Index-time full Zahir after enabling `enable_enhance_all` (background snapshot).
    pub force_full_enhance_background: &'static str,
    pub enhanced_with_zahirscan: &'static str,
    pub enhance_failed_prefix: &'static str,
    pub copied_path_to_clipboard: &'static str,
    pub copied_zahir_json_to_clipboard: &'static str,
    pub copy_path_failed_prefix: &'static str,
    pub copy_zahir_json_failed_prefix: &'static str,
    /// Placeholder `{LENS}` replaced with the lens name.
    pub removed_from_lens: &'static str,
    /// Replace `{PATH}` with the new relative path after rename.
    pub file_renamed: &'static str,
    pub file_deleted: &'static str,
    pub file_ops_failed_prefix: &'static str,
}

pub struct UiStringsLens {
    pub menu_create_new: &'static str,
    pub name_prompt: &'static str,
    pub rename_prompt: &'static str,
    pub delete_confirm_title: &'static str,
    pub delete_yes: &'static str,
    pub delete_no: &'static str,
    /// Replace `{LENS}` with the lens name (same pattern as other lens toasts).
    pub toast_created_and_added_file: &'static str,
    pub toast_added_to_lens: &'static str,
    pub toast_renamed_to: &'static str,
    pub toast_deleted_lens: &'static str,
}

/// Rename / delete entry under the indexed root (space menu file actions).
pub struct UiStringsFile {
    pub rename_prompt: &'static str,
    pub delete_confirm_title: &'static str,
}

/// Settings tab: bool row labels (TOML key names). For `show_hidden_files` / `hash` in the left pane, use [`append_settings_bool_snapshot_footnote`].
pub struct UiStringsSettingsBool {
    pub show_hidden_files: &'static str,
    pub hash: &'static str,
    pub enable_enhance_all: &'static str,
    pub ask_enhance_on_new_root: &'static str,
    pub unknown_row: &'static str,
}

/// Suffix for Settings left-pane rows whose values apply on the next snapshot (same `*` as the footnote under External apps).
pub const SETTINGS_BOOL_SNAPSHOT_FOOTNOTE_SUFFIX: &str = " *";

#[must_use]
pub fn append_settings_bool_snapshot_footnote(base: &'static str) -> String {
    format!("{base}{SETTINGS_BOOL_SNAPSHOT_FOOTNOTE_SUFFIX}")
}

/// First launch: no local `ublx.toml` yet.
pub struct UiStringsFirstRun {
    pub welcome_title: &'static str,
    pub root_choice_footer: &'static str,
    pub ublx_here: &'static str,
    pub recent_heading: &'static str,
    pub enhance_prompt_title: &'static str,
    /// Shown below Yes/No (hint style). `ublx.toml` / `.ublx.toml`: `enable_enhance_all`.
    pub enhance_prompt_footnote: &'static str,
    pub enhance_yes: &'static str,
    pub enhance_no: &'static str,
    pub previous_settings_title: &'static str,
    pub previous_settings_footnote: &'static str,
    pub previous_settings_use: &'static str,
    pub previous_settings_fresh: &'static str,
}

pub struct UiStringsSpaceMenu {
    pub open: &'static str,
    /// Reveal in Finder / Explorer, or open parent folder (Linux).
    pub show_in_folder: &'static str,
    pub copy_path: &'static str,
    /// Index-time batch Zahir for this directory subtree (`[[enhance_policy]]`); snapshot Directory rows only.
    pub enhance_policy: &'static str,
    pub enhance_policy_always: &'static str,
    pub enhance_policy_never: &'static str,
    /// Run full `ZahirScan` on this file when `enable_enhance_all` is false.
    pub enhance_with_zahirscan: &'static str,
    /// Copy raw snapshot `zahir_json` for this entry to the clipboard (space menu; only when JSON exists).
    pub copy_zahir_json: &'static str,
    pub add_to_lens: &'static str,
    pub remove_from_lens: &'static str,
    pub rename: &'static str,
    pub delete: &'static str,
}

/// All symbols and string literals used by the renderer.
pub struct UiStrings {
    pub loading: UiStringsLoading,
    pub delta: UiStringsDelta,
    pub pane: UiStringsPane,
    pub list: UiStringsList,
    pub main_tabs: UiStringsMainTabs,
    pub search: UiStringsSearchStatus,
    pub config: UiStringsConfig,
    pub paths: UiStringsPaths,
    pub brand: UiStringsBrand,
    pub tables: UiStringsTables,
    pub dialogs: UiStringsDialogs,
    pub viewer_raster: UiStringsViewerRaster,
    pub toasts: UiStringsToasts,
    pub lens: UiStringsLens,
    pub space: UiStringsSpaceMenu,
    pub file: UiStringsFile,
    pub first_run: UiStringsFirstRun,
    pub settings_bool: UiStringsSettingsBool,
}

impl Default for UiStrings {
    fn default() -> Self {
        Self::new()
    }
}

impl StringObjTraits for UiStrings {
    fn new() -> Self {
        UiStrings::new()
    }
}

impl UiStrings {
    const fn loading() -> UiStringsLoading {
        UiStringsLoading {
            general: "Loading…",
        }
    }

    const fn delta() -> UiStringsDelta {
        UiStringsDelta {
            added: "Added",
            modified: "Modified",
            removed: "Removed",
            right_title: "Snapshot overview",
            left_block_title: "Delta",
            placeholder_dash: "—",
            type_label: "Delta type",
        }
    }

    const fn pane() -> UiStringsPane {
        UiStringsPane {
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
            not_available: "(not available for this item)",
            viewer_placeholder: "(viewer — file content will load here)",
        }
    }

    const fn list() -> UiStringsList {
        UiStringsList {
            all_categories: "All",
            no_contents: "(no contents)",
            no_matches: "(no matches)",
            list_symbol: "  ",
        }
    }

    const fn main_tabs() -> UiStringsMainTabs {
        UiStringsMainTabs {
            snapshot: "Snapshot",
            delta: "Delta",
            settings: "Settings",
            duplicates: "Duplicates",
            lenses: "Lenses",
        }
    }

    const fn search() -> UiStringsSearchStatus {
        UiStringsSearchStatus {
            search_label: "Search (Categories & Contents): ",
            find_label: "Find: ",
            latest_snapshot: "Latest Snapshot",
        }
    }

    const fn config() -> UiStringsConfig {
        UiStringsConfig {
            global: "Global",
            local: "Local",
        }
    }

    const fn paths() -> UiStringsPaths {
        UiStringsPaths {
            paths: "Paths",
            duplicate_group: "Duplicate",
            lens_group: "Lens",
        }
    }

    const fn brand() -> UiStringsBrand {
        UiStringsBrand {
            brand: "UBLX",
            fullscreen_suffix: "(Esc to exit fullscreen)",
        }
    }

    const fn tables() -> UiStringsTables {
        UiStringsTables {
            header_key: "Key",
            header_value: "Value",
            first_title: "General",
            contents_title: "Contents",
            columns_title: "Columns",
        }
    }

    const fn dialogs() -> UiStringsDialogs {
        UiStringsDialogs {
            help: "Help",
            theme: "Theme",
            notification: "Notification",
            help_command: "Command",
            help_action: "Action",
        }
    }

    const fn viewer_raster() -> UiStringsViewerRaster {
        UiStringsViewerRaster {
            page_label: "Page",
            could_not_load_preview: "Could not load preview",
            could_not_decode_cover: "Could not decode cover",
            could_not_open_image: "Could not open image",
        }
    }

    const fn toasts() -> UiStringsToasts {
        UiStringsToasts {
            config_reloaded: "Config reloaded",
            no_duplicates: "No duplicates found",
            force_full_enhance_background: "Getting metadata for all files.",
            enhanced_with_zahirscan: "Enhanced with ZahirScan",
            enhance_failed_prefix: "Enhance failed: ",
            copied_path_to_clipboard: "Copied path to clipboard",
            copied_zahir_json_to_clipboard: "Copied Zahir JSON to clipboard",
            copy_path_failed_prefix: "Copy path failed: ",
            copy_zahir_json_failed_prefix: "Copy Zahir JSON failed: ",
            removed_from_lens: r#"Removed from lens "{LENS}""#,
            file_renamed: r#"Renamed to "{PATH}""#,
            file_deleted: "Deleted",
            file_ops_failed_prefix: "Failed: ",
        }
    }

    const fn file_strings() -> UiStringsFile {
        UiStringsFile {
            rename_prompt: "Rename to: ",
            delete_confirm_title: "Delete ",
        }
    }

    const fn lens() -> UiStringsLens {
        UiStringsLens {
            menu_create_new: "Create New Lens",
            name_prompt: "Lens name: ",
            rename_prompt: "Rename lens: ",
            delete_confirm_title: "Delete lens ",
            delete_yes: "Yes (y)",
            delete_no: "No (n)",
            toast_created_and_added_file: r#"Created lens "{LENS}" and added file"#,
            toast_added_to_lens: r#"Added to lens "{LENS}""#,
            toast_renamed_to: r#"Renamed lens to "{LENS}""#,
            toast_deleted_lens: r#"Deleted lens "{LENS}""#,
        }
    }

    const fn space() -> UiStringsSpaceMenu {
        UiStringsSpaceMenu {
            open: "Open",
            show_in_folder: "Show in folder",
            copy_path: "Copy Path",
            enhance_policy: "Enhance policy",
            enhance_policy_always: "Always — automatic (y)",
            enhance_policy_never: "Per-file — manual (n)",
            enhance_with_zahirscan: "Enhance with ZahirScan",
            copy_zahir_json: "Copy Templates",
            add_to_lens: "Add to Lens",
            remove_from_lens: "Remove from Lens",
            rename: "Rename",
            delete: "Delete",
        }
    }

    const fn settings_bool() -> UiStringsSettingsBool {
        UiStringsSettingsBool {
            show_hidden_files: "show_hidden_files",
            hash: "hash",
            enable_enhance_all: "enable_enhance_all",
            ask_enhance_on_new_root: "ask_enhance_on_new_root",
            unknown_row: "?",
        }
    }

    const fn first_run() -> UiStringsFirstRun {
        UiStringsFirstRun {
            welcome_title: "Welcome to UBLX",
            root_choice_footer: "Enter — confirm   Esc / q — quit",
            ublx_here: "New UBLX here: ",
            recent_heading: "Recent UBLX",
            enhance_prompt_title: "Index with full ZahirScan for all files automatically?",
            enhance_prompt_footnote: "Not recommended for very large directories.\nChange anytime in Settings (`enable_enhance_all`).\nTo turn off this prompt: `ask_enhance_on_new_root = false` in Settings (Global).\nDefault is off unless you set `enable_enhance_all = true`.",
            enhance_yes: "Yes (y)",
            enhance_no: "No (n)",
            previous_settings_title: "Previous settings found",
            previous_settings_footnote: "Use saved: keep or restore `ublx.toml` from the last run.\nStart fresh: remove local and cached config, then continue setup.\nGlobal config remains unchanged.",
            previous_settings_use: "Use saved settings (y)",
            previous_settings_fresh: "Start from scratch (n)",
        }
    }

    #[must_use]
    pub const fn new() -> Self {
        Self {
            loading: Self::loading(),
            delta: Self::delta(),
            pane: Self::pane(),
            list: Self::list(),
            main_tabs: Self::main_tabs(),
            search: Self::search(),
            config: Self::config(),
            paths: Self::paths(),
            brand: Self::brand(),
            tables: Self::tables(),
            dialogs: Self::dialogs(),
            viewer_raster: Self::viewer_raster(),
            toasts: Self::toasts(),
            lens: Self::lens(),
            space: Self::space(),
            file: Self::file_strings(),
            first_run: Self::first_run(),
            settings_bool: Self::settings_bool(),
        }
    }

    // /// Toast when config is reloaded by file watcher (save).
    // #[must_use]
    // pub fn config_reload_triggered_by_save(&self) -> String {
    //     format!("{} (triggered by save)", self.toasts.config_reloaded)
    // }

    /// Padded label width so **Dark** / **Light** section lines share the same total width.
    pub const THEME_SELECTOR_SECTION_LABEL_WIDTH: usize = 5;

    /// Theme picker section row: indent, left rule, padded label, spaces, right rule. **Dark** uses a single space before the right rule and one extra trailing `─` (see [`Self::theme_selector_section_row_dark`]).
    #[must_use]
    pub fn theme_selector_section_row(&self, label: &str) -> String {
        if label == "Dark" {
            self.theme_selector_section_row_dark()
        } else {
            format!(
                "   {} {:width$} {}",
                UI_GLYPHS.theme_section_rule,
                label,
                UI_GLYPHS.theme_section_rule,
                width = Self::THEME_SELECTOR_SECTION_LABEL_WIDTH
            )
        }
    }

    /// Dark section: `───` + padded `Dark` + `───` + `─` (no extra space before the right rule; avoids pad + separator doubling).
    #[must_use]
    pub fn theme_selector_section_row_dark(&self) -> String {
        format!(
            "   {} {:width$}{}{}",
            UI_GLYPHS.theme_section_rule,
            "Dark",
            UI_GLYPHS.theme_section_rule,
            '\u{2500}',
            width = Self::THEME_SELECTOR_SECTION_LABEL_WIDTH
        )
    }

    /// Display width of a section row (same for Dark and Light layouts; keep in sync with [`Self::theme_selector_section_row`]).
    #[must_use]
    pub fn theme_selector_section_row_width(&self) -> usize {
        3 + 2 * UI_GLYPHS.theme_section_rule.chars().count()
            + 1
            + Self::THEME_SELECTOR_SECTION_LABEL_WIDTH
            + 1
    }

    /// PDF page footer (`Page 2 / 10` or `Page 2` when total unknown).
    #[must_use]
    pub fn viewer_pdf_page_footer(&self, page: u32, page_count: Option<u32>) -> String {
        let p = page.max(1);
        match page_count {
            Some(n) => format!("{} {p} / {n}", self.viewer_raster.page_label),
            None => format!("{} {p}", self.viewer_raster.page_label),
        }
    }

    #[must_use]
    pub fn viewer_err_load_preview(&self, e: impl std::fmt::Display) -> String {
        format!("{}: {}", self.viewer_raster.could_not_load_preview, e)
    }

    #[must_use]
    pub fn viewer_err_decode_cover(&self, e: impl std::fmt::Display) -> String {
        format!("{}: {}", self.viewer_raster.could_not_decode_cover, e)
    }

    #[must_use]
    pub fn viewer_err_open_image(&self, e: impl std::fmt::Display) -> String {
        format!("{}: {}", self.viewer_raster.could_not_open_image, e)
    }
}

pub const UI_STRINGS: UiStrings = UiStrings::new();

/// Shared UI layout constants (padding, etc.). Constraint arrays are derived from the scalar values via the `*_constraints()` methods.
pub struct UiConstants {
    pub h_pad: u16,
    pub v_pad: u16,
    pub popup_padding_w: u16,
    pub popup_padding_h: u16,
    /// Theme-picker swatch for **dark** themes on a **dark** popup: HSL lighten off page background via [`crate::themes::adjust_surface_rgb`].
    pub swatch_lighten: f32,
    /// Same as [`Self::swatch_lighten`] but when the picker is shown while a **light** theme is active — stronger lighten so chips are not mud-on-cream.
    pub swatch_lighten_dark_on_light_popup: f32,
    /// Theme-picker swatch for **light** themes: [`crate::themes::lighten_rgb`] on body text (try 0.2–0.4).
    pub swatch_light_theme_text: f32,
    pub table_stripe_lighten: f32,
    pub input_poll_ms: u64,
    pub status_line_height: u16,
    pub tab_row_height: u16,
    pub brand_block_width: u16,
    pub empty_space: &'static str,
}

impl Default for UiConstants {
    fn default() -> Self {
        Self::new()
    }
}

impl UiConstants {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            h_pad: 1,
            v_pad: 1,
            popup_padding_w: 4,
            popup_padding_h: 2,
            swatch_lighten: 0.2,
            swatch_lighten_dark_on_light_popup: 0.38,
            swatch_light_theme_text: 0.6,
            table_stripe_lighten: 0.06,
            input_poll_ms: 100,
            status_line_height: 1,
            tab_row_height: 1,
            brand_block_width: 4,
            empty_space: " ",
        }
    }

    /// Main area (min 1 row) + status line. Derived from [`Self::status_line_height`].
    #[must_use]
    pub fn status_line_constraints(&self) -> [Constraint; 2] {
        [
            Constraint::Min(1),
            Constraint::Length(self.status_line_height),
        ]
    }

    /// Tab row (Snapshot|Delta) + body. Derived from [`Self::tab_row_height`].
    #[must_use]
    pub fn tab_row_constraints(&self) -> [Constraint; 2] {
        [Constraint::Length(self.tab_row_height), Constraint::Min(1)]
    }

    /// Tabs (flex) + brand block. Derived from [`Self::brand_block_width`].
    #[must_use]
    pub fn brand_block_constraints(&self) -> [Constraint; 2] {
        [
            Constraint::Min(0),
            Constraint::Length(self.brand_block_width),
        ]
    }

    #[must_use]
    pub fn get_empty_span(&self, style: Style) -> Span<'static> {
        Span::styled(self.empty_space, style)
    }
}

pub const UI_CONSTANTS: UiConstants = UiConstants::new();

/// Unicode symbols used in layout/render. Nerd Fonts or similar may be needed for powerline characters.
pub struct UiGlyphs {
    /// Powerline-style segment: round left (curve on right). Used for tab nodes and status nodes.
    pub round_left: char,
    /// Powerline-style segment: round right (curve on left). Used for tab nodes and status nodes.
    pub round_right: char,
    /// Full block (e.g. theme selector swatch). U+2588.
    pub swatch_block: char,
    /// Short box-drawing run for theme-picker section headers (`───`); placed on both sides of **Dark** / **Light**.
    pub theme_section_rule: &'static str,
    /// Markdown viewer: suffix (after link text) for inline links `[text](url)`.
    pub markdown_link: char,
    /// Markdown viewer: suffix for link destinations that look like file attachments (.pdf, .zip, …).
    pub markdown_attachment: char,
    /// Markdown viewer: prefix for `![alt](url)` image syntax (photo / figure).
    pub markdown_image: char,
    /// Sort direction glyph for ascending/up.
    pub arrow_up: char,
    /// Sort direction glyph for descending/down.
    pub arrow_down: char,
    /// Settings left pane: prefix when this row is focused (`›` + space).
    pub settings_row_active: &'static str,
    /// Two-space indent: inactive Settings row prefix and wrapped path continuation lines.
    pub indent_two_spaces: &'static str,
}

impl Default for UiGlyphs {
    fn default() -> Self {
        Self::new()
    }
}

impl StringObjTraits for UiGlyphs {
    fn new() -> Self {
        UiGlyphs::new()
    }
}

impl UiGlyphs {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            round_left: '\u{e0b6}',
            round_right: '\u{e0b4}',
            swatch_block: '\u{2588}',
            theme_section_rule: "\u{2500}\u{2500}\u{2500}",
            markdown_link: '\u{2197}',        // ↗
            markdown_attachment: '\u{1f4ce}', // 📎
            markdown_image: '\u{1f5bc}',      // 🖼 (framed picture)
            arrow_up: '\u{2191}',             // ↑
            arrow_down: '\u{2193}',           // ↓
            settings_row_active: "\u{203a} ", // ›
            indent_two_spaces: "  ",
        }
    }
}

pub const UI_GLYPHS: UiGlyphs = UiGlyphs::new();

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

impl Default for TreeChars {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeChars {
    #[must_use]
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
