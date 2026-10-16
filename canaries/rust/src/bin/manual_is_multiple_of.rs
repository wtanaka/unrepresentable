//! Canary.
/// Canary item.
const fn multiple(value: u32, divisor: u32) -> bool {
    value % divisor == 0 //~ clippy::arithmetic_side_effects clippy::integer_division_remainder_used clippy::manual_is_multiple_of
}

fn main() {
    assert!(multiple(4, 2), "multiple");
}
