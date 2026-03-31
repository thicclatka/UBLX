//! Main tab bar mode order and labels (Snapshot, optional Lenses, Delta, optional Duplicates, Settings).

use ublx::layout::setup::MainMode;
use ublx::ui::main_tab_bar_modes_and_labels;

#[test]
fn main_tab_bar_modes_order_no_optional() {
    let (modes, labels) = main_tab_bar_modes_and_labels(false, false);
    assert_eq!(
        modes,
        vec![MainMode::Snapshot, MainMode::Delta, MainMode::Settings,]
    );
    assert_eq!(modes.len(), labels.len());
}

#[test]
fn main_tab_bar_modes_order_all_optional() {
    let (modes, labels) = main_tab_bar_modes_and_labels(true, true);
    assert_eq!(
        modes,
        vec![
            MainMode::Snapshot,
            MainMode::Lenses,
            MainMode::Delta,
            MainMode::Duplicates,
            MainMode::Settings,
        ]
    );
    assert_eq!(modes.len(), labels.len());
}

#[test]
fn main_tab_bar_modes_lenses_only() {
    let (modes, _) = main_tab_bar_modes_and_labels(true, false);
    assert_eq!(
        modes,
        vec![
            MainMode::Snapshot,
            MainMode::Lenses,
            MainMode::Delta,
            MainMode::Settings,
        ]
    );
}

#[test]
fn main_tab_bar_modes_duplicates_only() {
    let (modes, _) = main_tab_bar_modes_and_labels(false, true);
    assert_eq!(
        modes,
        vec![
            MainMode::Snapshot,
            MainMode::Delta,
            MainMode::Duplicates,
            MainMode::Settings,
        ]
    );
}
