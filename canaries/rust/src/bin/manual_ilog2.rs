//! Canary.
/// Canary item.
const fn log(value: u32) -> u32 {
    31 - value.leading_zeros() //~ clippy::arithmetic_side_effects clippy::manual_ilog2
}

fn main() {
    assert_eq!(log(8), 3, "log");
}
