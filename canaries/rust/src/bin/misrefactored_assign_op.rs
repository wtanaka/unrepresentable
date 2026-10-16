//! Canary.
fn main() {
    let mut value: u8 = 1;
    value += value + 1; //~ clippy::arithmetic_side_effects clippy::misrefactored_assign_op
    assert_eq!(value, 3, "value");
}
