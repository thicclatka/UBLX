//! Job identity and channel state for background viewer rendering ([`crate::render::viewers::async_tools`]).

use std::sync::mpsc;

use crate::engine::cache::{ViewerTableCacheKey, ViewerTextCacheEntry};

/// Which heavy viewer path a background job represents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewerAsyncJobKind {
    Markdown,
    Code,
    Csv,
}

/// Stable key for matching completed work to the current selection and layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerAsyncJobKey {
    pub path: String,
    pub content_width: u16,
    pub theme_name: String,
    pub kind: ViewerAsyncJobKind,
}

/// Result from a background viewer job.
pub enum ViewerAsyncResult {
    Markdown(ViewerTextCacheEntry),
    Code(ViewerTextCacheEntry),
    /// `None` if parse/layout failed on the worker (pending cleared; UI may retry sync).
    Csv(ViewerTableCacheKey, Option<ViewerTextCacheEntry>),
}

pub struct ViewerAsyncDone {
    pub key: ViewerAsyncJobKey,
    pub result: ViewerAsyncResult,
}

/// Pending background viewer work ([`crate::render::viewers::async_tools`]).
#[derive(Default)]
pub struct ViewerAsyncState {
    /// Last scheduled job identity; avoids duplicate spawns for the same key.
    pub pending_key: Option<ViewerAsyncJobKey>,
    pub rx: Option<mpsc::Receiver<ViewerAsyncDone>>,
}

impl ViewerAsyncState {
    pub fn clear(&mut self) {
        self.pending_key = None;
        self.rx = None;
    }
}
