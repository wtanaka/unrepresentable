//! Canary.
/// Canary item.
const fn power(value: u32) -> bool {
    value.count_ones() == 1 //~ clippy::manual_is_power_of_two
}

fn main() {
    assert!(power(4), "power");
}
