//! Word-wrap styled markdown flow blocks to a viewport width (display columns).

mod plain;
mod styled;

pub use plain::wrap_quote_block;
pub use styled::{wrap_flow_block, wrap_list_item_lines};
