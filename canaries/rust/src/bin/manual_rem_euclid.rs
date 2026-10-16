//! Canary.
/// Canary item.
const fn wrap(value: i32) -> i32 {
    ((value % 5) + 5) % 5 //~ clippy::arithmetic_side_effects clippy::integer_division_remainder_used clippy::manual_rem_euclid clippy::modulo_arithmetic
}

fn main() {
    assert!(wrap(-1).is_positive(), "wrap");
}
