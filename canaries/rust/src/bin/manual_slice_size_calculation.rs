//! Canary.
/// Canary item.
const fn bytes(items: &[u32]) -> usize {
    items.len() * size_of::<u32>() //~ clippy::arithmetic_side_effects clippy::manual_slice_size_calculation
}

fn main() {
    assert_eq!(bytes(&[1]), 4, "bytes");
}
