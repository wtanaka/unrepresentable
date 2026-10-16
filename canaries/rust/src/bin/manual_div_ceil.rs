//! Canary.
/// Canary item.
const fn blocks(len: u32) -> u32 {
    (len + 7) / 8 //~ clippy::arithmetic_side_effects clippy::integer_division clippy::integer_division_remainder_used clippy::manual_div_ceil
}

fn main() {
    assert_eq!(blocks(9), 2, "blocks");
}
