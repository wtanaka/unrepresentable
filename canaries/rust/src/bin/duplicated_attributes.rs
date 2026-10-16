//! Canary.
#[rustfmt::skip]
#[must_use]
#[must_use] //~ clippy::duplicated_attributes unused_attributes
/// Canary item.
fn value() -> u8 { //~ clippy::missing_const_for_fn
    1
}

fn main() {
    assert!(value() == 1, "one");
}
