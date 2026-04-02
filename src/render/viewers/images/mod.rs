//! Image and PDF raster preview (`ratatui-image`): policy in [`raster_policy`], wiring in [`core`].

mod core;
mod raster_policy;

pub use core::*;
pub use raster_policy::{
    VIEWPORT_RASTER_IMAGE, VIEWPORT_RASTER_PDF, ViewportCellRasterBudget, downscale_with_max,
    max_edge_for_pdf_viewport_cells, max_edge_for_viewport_cells,
    tiered_max_dimension_for_file_size,
};
