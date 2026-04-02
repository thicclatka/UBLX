//! Bumper buffer (`BumperBuffer`) behavior.

use log::Level;
use ublx::utils::BumperBuffer;

#[test]
fn remove_messages_for_operation_drops_matching_only() {
    let b = BumperBuffer::new(100);
    b.push_with_operation(Level::Info, "snap", Some("ublx: snapshot"));
    b.push_with_operation(Level::Info, "lens", Some("ublx: lens"));
    b.remove_messages_for_operation("ublx: snapshot");
    let last = b.last_n(10);
    assert_eq!(last.len(), 1);
    assert_eq!(last[0].text, "lens");
}
