//! Snapshot loading for the TUI (categories + rows, sorted).

use std::path::Path;

use crate::engine::db_ops;
use crate::layout::setup;

fn sort_categories_and_rows(categories: &mut [String], all_rows: &mut [setup::TuiRow]) {
    categories.sort();
    all_rows.sort_by(|a, b| a.0.cmp(&b.0));
}

/// Load snapshot categories and rows for the TUI, sorted. Use at startup and when a snapshot run completes.
pub fn load_snapshot_for_tui(db_path: &Path) -> (Vec<String>, Vec<setup::TuiRow>) {
    let mut categories = db_ops::load_snapshot_categories(db_path).unwrap_or_default();
    let mut all_rows = db_ops::load_snapshot_rows_for_tui(db_path, None).unwrap_or_default();
    sort_categories_and_rows(&mut categories, &mut all_rows);
    (categories, all_rows)
}
