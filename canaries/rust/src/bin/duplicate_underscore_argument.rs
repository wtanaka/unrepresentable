//! Canary.
#[rustfmt::skip]
/// Canary item.
fn combine(first: u8, _first: u8) -> u8 { //~ clippy::duplicate_underscore_argument clippy::missing_const_for_fn
    first
}

fn main() {
    assert!(combine(1, 2) == 1, "first");
}
