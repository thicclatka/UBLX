pub struct ToastConfig {
    pub width: u16,
    pub height: u16,
    pub hz_padding: u16,
    pub vt_padding: u16,
    pub duration: std::time::Duration,
    pub display_lines: usize,
    pub theme_selector_height: u16,
    pub theme_selector_display_lines: usize,
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

/// Pick `dev_val` when `dev` is true, else `normal`. Used by ToastConfig `*_for(dev)` methods.
fn pick<T>(dev: bool, normal: T, dev_val: T) -> T {
    if dev { dev_val } else { normal }
}

impl Default for ToastConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl ToastConfig {
    pub const fn new() -> Self {
        Self {
            width: 44,
            height: 4,
            hz_padding: 1,
            vt_padding: 2,
            duration: std::time::Duration::from_secs(4),
            display_lines: 2,
            theme_selector_height: 3,
            theme_selector_display_lines: 1,
            bumper_cap: 100,
            dev_width: 100,
            dev_height: 20,
            dev_display_lines: 10,
            dev_bumper_cap: 500,
            max_toast_stack: 3,
            toast_stack_gap: 1,
        }
    }

    pub fn bumper_cap_for(&self, dev: bool) -> usize {
        pick(dev, self.bumper_cap, self.dev_bumper_cap)
    }

    pub fn width_for(&self, dev: bool) -> u16 {
        pick(dev, self.width, self.dev_width)
    }

    pub fn height_for(&self, dev: bool) -> u16 {
        pick(dev, self.height, self.dev_height)
    }

    pub fn display_lines_for(&self, dev: bool) -> usize {
        pick(dev, self.display_lines, self.dev_display_lines)
    }

    /// Line count for the toast body: theme-selector uses 1 line only; others use display_lines_for(dev).
    pub fn display_lines_for_operation(&self, dev: bool, operation: Option<&str>) -> usize {
        if operation == Some(OPERATION_NAME.theme_selector()) {
            self.theme_selector_display_lines
        } else {
            self.display_lines_for(dev)
        }
    }

    pub fn height_for_operation(&self, dev: bool, operation: Option<&str>) -> u16 {
        if operation == Some(OPERATION_NAME.theme_selector()) {
            self.theme_selector_height
        } else {
            self.height_for(dev)
        }
    }
}

pub const TOAST_CONFIG: ToastConfig = ToastConfig::new();

/// Operation names for toasts: `{executable}-{op}` (e.g. "ublx-snapshot", "ublx-export").
pub struct OperationName {
    executable: &'static str,
}

impl Default for OperationName {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationName {
    pub const fn new() -> Self {
        Self {
            executable: env!("CARGO_PKG_NAME"),
        }
    }

    pub fn snapshot(&self) -> String {
        format!("{}-snapshot", self.executable)
    }

    /// Theme selector toast: operation name is just "theme-selector" (no executable prefix).
    pub fn theme_selector(&self) -> &'static str {
        "theme-selector"
    }

    /// For future operations: e.g. `op("export")` → "ublx-export".
    #[allow(dead_code)]
    pub fn op(&self, name: &str) -> String {
        format!("{}-{}", self.executable, name)
    }
}

pub const OPERATION_NAME: OperationName = OperationName::new();
