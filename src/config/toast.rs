/// Chars to subtract from toast width for borders + level prefix (e.g. ` [W] `) when word-wrapping bumper messages.
pub const TOAST_CONTENT_WIDTH_OFFSET: usize = 7;

pub struct ToastConfig {
    pub width: u16,
    /// Default max height (rows). Toast height is derived from content (\\n breaks + message count), clamped to this.
    pub height: u16,
    /// Fixed rows (e.g. top border+title, bottom border) added to content lines when computing toast height.
    pub toast_height_offset: u16,
    /// Minimum toast height in rows (content + offset is clamped to at least this).
    pub toast_height_min: u16,
    pub hz_padding: u16,
    pub vt_padding: u16,
    pub duration: std::time::Duration,
    pub display_lines: usize,
    pub bumper_cap: usize,
    /// Dev mode: larger toast and more lines.
    pub dev_width: u16,
    pub dev_height: u16,
    pub dev_display_lines: usize,
    pub dev_bumper_cap: usize,
    /// Max number of toasts to show stacked at once (oldest dropped when exceeded).
    pub max_toast_stack: usize,
    /// Vertical gap (rows) between stacked toasts.
    pub toast_stack_gap: u16,
}

/// Pick `dev_val` when `dev` is true, else `normal`. Used by `ToastConfig` `*_for(dev)` methods.
fn pick<T>(dev: bool, normal: T, dev_val: T) -> T {
    if dev { dev_val } else { normal }
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            width: 44,
            height: 10,
            toast_height_offset: 2,
            toast_height_min: 3,
            hz_padding: 1,
            vt_padding: 2,
            duration: std::time::Duration::from_secs(4),
            display_lines: 2,
            bumper_cap: 100,
            dev_width: 100,
            dev_height: 20,
            dev_display_lines: 10,
            dev_bumper_cap: 500,
            max_toast_stack: 3,
            toast_stack_gap: 1,
        }
    }

    #[must_use]
    pub fn bumper_cap_for(&self, dev: bool) -> usize {
        pick(dev, self.bumper_cap, self.dev_bumper_cap)
    }

    #[must_use]
    pub fn width_for(&self, dev: bool) -> u16 {
        pick(dev, self.width, self.dev_width)
    }

    #[must_use]
    pub fn height_for(&self, dev: bool) -> u16 {
        pick(dev, self.height, self.dev_height)
    }

    #[must_use]
    pub fn display_lines_for(&self, dev: bool) -> usize {
        pick(dev, self.display_lines, self.dev_display_lines)
    }

    /// Number of bumper messages to show in a toast. Height is derived from content (see [`crate::utils::notifications::toast_content_line_count`]).
    #[must_use]
    pub fn display_lines_for_operation(&self, dev: bool, _operation: Option<&str>) -> usize {
        self.display_lines_for(dev)
    }

    /// Width available for message text when wrapping (width minus [`TOAST_CONTENT_WIDTH_OFFSET`]).
    #[must_use]
    pub fn content_width_for(&self, dev: bool) -> usize {
        (self.width_for(dev) as usize).saturating_sub(TOAST_CONTENT_WIDTH_OFFSET)
    }
}

pub const TOAST_CONFIG: ToastConfig = ToastConfig::new();

/// Operation names for toasts and bumper grouping: `{executable}: {name}` (e.g. `ublx: snapshot`, `ublx: lens`).
pub struct OperationName {
    executable: &'static str,
}

impl Default for OperationName {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationName {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            executable: env!("CARGO_PKG_NAME"),
        }
    }

    /// e.g. `op("snapshot")` → `"ublx: snapshot"`, `op("theme-selector")` → `"ublx: theme-selector"`.
    #[must_use]
    pub fn op(&self, name: &str) -> String {
        format!("{}: {}", self.executable, name)
    }
}

pub const OPERATION_NAME: OperationName = OperationName::new();
