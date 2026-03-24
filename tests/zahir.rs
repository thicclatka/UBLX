use ublx::handlers::zahir_ops::{ZahirFileType as FileType, file_type_from_metadata_name};

#[test]
fn wrapper_matches_zahirscan_api() {
    assert_eq!(file_type_from_metadata_name("CSV"), Some(FileType::Csv));
    assert_eq!(
        file_type_from_metadata_name("Markdown"),
        Some(FileType::Markdown)
    );
}

#[test]
fn non_zahir_categories_miss() {
    assert_eq!(file_type_from_metadata_name("Directory"), None);
    assert_eq!(file_type_from_metadata_name("not a label"), None);
}
