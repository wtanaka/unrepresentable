//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn divide(left: u32, right: u32) -> Option<u32> {
    if right != 0 { //~ clippy::manual_checked_ops
        Some(left / right) //~ clippy::arithmetic_side_effects clippy::integer_division clippy::integer_division_remainder_used
    } else {
        None
    }
}

fn main() {
    assert_eq!(divide(4, 2), Some(2), "div");
}
