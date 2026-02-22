use serde_json::{Map, Value};

use crate::ui::TREE_CHARS;

/// Blank lines between stacked tables.
pub const TABLE_GAP: u16 = 1;

/// JSON keys that trigger special section handling (not treated as plain key/value or nested subsection).
pub struct SectionKeys;
impl SectionKeys {
    pub const SCHEMA: &'static str = "schema";
    pub const ENTRIES: &'static str = "entries";
    pub const SHEET_STATS: &'static str = "sheet_stats";
    pub const COMMON_PIVOTS: &'static str = "common_pivots";
    pub const CSV_METADATA: &'static str = "csv_metadata";
    /// Array of table objects (e.g. sqlite_metadata.tables); each object becomes its own KV section with title from "name".
    pub const TABLES: &'static str = "tables";
}

/// Keys used inside schema tree values (attributes, children). Not top-level section keys.
pub struct SchemaKeys;
impl SchemaKeys {
    pub const ATTRIBUTES: &'static str = "attributes";
    pub const CHILDREN: &'static str = "children";

    pub fn has_attributes(map: &Map<String, Value>) -> bool {
        map.contains_key(SchemaKeys::ATTRIBUTES)
    }

    pub fn has_children(map: &Map<String, Value>) -> bool {
        map.contains_key(SchemaKeys::CHILDREN)
    }

    #[inline]
    pub fn has_children_or_attributes(map: &Map<String, Value>) -> bool {
        SchemaKeys::has_attributes(map) || SchemaKeys::has_children(map)
    }
}

/// Returns (branch_line_prefix, continuation_prefix) for the next level. Use branch for the current line, continuation for recursing.
pub fn tree_prefixes(continuation: &str, is_last: bool) -> (String, String) {
    let branch = if is_last {
        TREE_CHARS.last_branch
    } else {
        TREE_CHARS.branch
    };
    let cont = if is_last {
        TREE_CHARS.space
    } else {
        TREE_CHARS.vertical
    };
    (
        format!("{continuation}{branch}"),
        format!("{continuation}{cont}"),
    )
}
