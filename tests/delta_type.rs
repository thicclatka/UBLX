//! Tests for engine::db_ops::DeltaType index mapping.

use ublx::engine::db_ops::{DELTA_CATEGORY_COUNT, DeltaType};

#[test]
fn delta_type_as_index() {
    assert_eq!(DeltaType::Added.as_index(), 0);
    assert_eq!(DeltaType::Mod.as_index(), 1);
    assert_eq!(DeltaType::Removed.as_index(), 2);
}

#[test]
fn delta_type_from_index() {
    assert_eq!(DeltaType::from_index(0), DeltaType::Added);
    assert_eq!(DeltaType::from_index(1), DeltaType::Mod);
    assert_eq!(DeltaType::from_index(2), DeltaType::Removed);
    assert_eq!(DeltaType::from_index(3), DeltaType::Removed);
    assert_eq!(DeltaType::from_index(100), DeltaType::Removed);
}

#[test]
fn delta_type_roundtrip() {
    for (i, t) in DeltaType::iter().enumerate() {
        assert_eq!(t.as_index(), i);
        assert_eq!(DeltaType::from_index(i), t);
    }
}

#[test]
fn delta_category_count() {
    assert_eq!(DELTA_CATEGORY_COUNT, 3);
    assert_eq!(DeltaType::iter().count(), DELTA_CATEGORY_COUNT);
}
