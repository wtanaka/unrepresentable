//! Canary.
/// Canary item.
const fn lowest(value: u32) -> u32 {
    value & value.wrapping_neg() //~ clippy::manual_isolate_lowest_one
}

fn main() {
    assert_eq!(lowest(6), 2, "lowest");
}
