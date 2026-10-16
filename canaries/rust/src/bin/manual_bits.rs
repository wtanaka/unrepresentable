//! Canary.
/// Canary item.
const fn width() -> usize {
    size_of::<u8>() * 8 //~ clippy::arithmetic_side_effects clippy::manual_bits
}

fn main() {
    assert_eq!(width(), 8, "bits");
}
