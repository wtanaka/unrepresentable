//! Canary.
/// Canary item.
fn zip(left: Option<u8>, right: Option<u8>) -> Option<(u8, u8)> {
    left.and_then(|lhs| right.map(|rhs| (lhs, rhs))) //~ clippy::manual_option_zip clippy::return_and_then
}

fn main() {
    assert_eq!(zip(Some(1), Some(2)), Some((1, 2)), "zip");
}
