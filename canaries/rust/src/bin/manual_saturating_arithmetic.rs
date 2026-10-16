//! Canary.
/// Canary item.
fn sat(left: u32, right: u32) -> u32 {
    left.checked_add(right).unwrap_or(u32::MAX) //~ clippy::manual_saturating_arithmetic
}

fn main() {
    assert_eq!(sat(1, 2), 3, "sat");
}
