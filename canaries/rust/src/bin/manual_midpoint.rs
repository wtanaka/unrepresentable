//! Canary.
/// Canary item.
const fn mid(left: u32, right: u32) -> u32 {
    (left + right) / 2 //~ clippy::arithmetic_side_effects clippy::integer_division clippy::integer_division_remainder_used clippy::manual_midpoint
}

fn main() {
    assert_eq!(mid(2, 4), 3, "mid");
}
