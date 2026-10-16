//! Canary.
/// Canary item.
fn check(left: u8, right: u8) {
    assert!(left == right); //~ clippy::manual_assert_eq clippy::missing_assert_message
}

fn main() {
    check(1, 1);
}
