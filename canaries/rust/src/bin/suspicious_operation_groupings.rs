//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Pair {
    /// Canary item.
    left: u8,
    /// Canary item.
    right: u8,
}

#[rustfmt::skip]
fn main() {
    let first = Pair { left: 1, right: 2 };
    let second = Pair { left: 1, right: 2 };
    assert!(first.left == second.left && first.right == second.left, "same"); //~ clippy::suspicious_operation_groupings
}
