//! Canary.
/// Canary item.
fn sum(items: &[u8]) -> u8 {
    items[0].max(items[1]).max(items[2]) //~ clippy::indexing_slicing clippy::missing_asserts_for_indexing
}

fn main() {
    assert_eq!(sum(&[1, 2, 3]), 3, "sum");
}
