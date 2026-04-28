//! Right-hand preview: [`core`] (tree + file viewer + zahir sectioning), [`zarrstore`] (Zarr store
//! directory rows), [`async_ops`] (off-thread resolve for file and Zarr store selections).

pub mod async_ops;
mod core;
pub mod zarrstore;

pub use core::*;
pub use zarrstore::{ZarrStoreRightPaneBuild, ZarrStoreRightPaneView, build_zarr_store_right_pane};
