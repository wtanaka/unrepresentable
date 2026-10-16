//! Canary.
#[rustfmt::skip]
/// Canary item.
fn inside(value: u32) -> bool { //~ clippy::missing_const_for_fn
    value >= 1 && value < 5 //~ clippy::manual_range_contains
}

fn main() {
    assert!(inside(2), "inside");
}
