//! Parallel iteration thresholds. Used by layout filter, user-selected mode, CSV viewer,
//! kv_tables sections, tables natural widths, and markdown to decide when to use rayon.

/// Thresholds above which we switch to parallel iteration for specific operations.
#[derive(Clone, Copy, Debug)]
pub struct Parallel {
    /// Snapshot content indices: filter rows by category + search. (`layout::filter::content_indices_for_view`)
    pub content_indices: usize,
    /// Categories that have ≥1 row matching search. (`layout::filter::categories_for_search`)
    pub categories_for_search: usize,
    /// Delta rows filtered by path containing query. (`layout::filter::filter_delta_rows`)
    pub delta_rows: usize,
    /// Duplicates groups or Lenses names filtered by search. (`layout::event_loop::user_selected`)
    pub user_selected_filter: usize,
    /// CSV viewer: truncate table cells per row. (`render::viewers::csv::truncate_all_cells`)
    pub csv_truncate: usize,
    /// JSON metadata: parse each "\n\n" blob in parallel. (`render::kv_tables::sections::parse_json_sections`)
    pub json_sections_blobs: usize,
    /// Contents table: natural column widths over visible rows (`kv_tables::ratatui_table`, internal).
    pub contents_natural_widths: usize,
    /// Markdown viewer: render blocks to lines in parallel. (`render::viewers::markdown::MarkdownDoc::to_text`)
    pub markdown_blocks: usize,
    /// Snapshot insert: parallelize preparation (path_str, category, zahir_json, mtime_ns, size, hash); insert stays sequential. (`engine::db_ops::utils::insert_results_into_snapshot`)
    pub snapshot_insert_prep: usize,
    /// Paths that need zahir: filter nefax by size > 0 and mtime/changed. (`engine::orchestrator::paths_needing_zahir`)
    pub paths_needing_zahir: usize,
}

/// Default parallel thresholds used across the app.
pub const PARALLEL: Parallel = Parallel {
    content_indices: 5000,
    categories_for_search: 100,
    delta_rows: 2000,
    user_selected_filter: 500,
    csv_truncate: 100,
    json_sections_blobs: 20,
    contents_natural_widths: 1000,
    markdown_blocks: 100,
    snapshot_insert_prep: 5000,
    paths_needing_zahir: 20_000,
};
