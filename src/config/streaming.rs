//! Stream vs sequential run mode: branch pipeline on [RunMode] (from [UblxOpts]).

use crate::config::UblxOpts;

/// Whether to run the index → zahir pipeline in streaming (callback/channel) or sequential (nefax then zahir on full set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunMode {
    /// Nefax runs with entry callback; paths stream to zahir (e.g. channel). Overlapped work.
    Stream,
    /// Nefax runs to completion; then zahir runs on the full path set. One phase after the other.
    Sequential,
}

impl RunMode {
    /// Derive mode from opts: [UblxOpts::streaming] ⇒ Stream, else Sequential.
    pub fn from_opts(opts: &UblxOpts) -> Self {
        if opts.streaming {
            Self::Stream
        } else {
            Self::Sequential
        }
    }
}
