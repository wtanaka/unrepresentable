//! Canary.
#[must_use] //~ clippy::empty_line_after_outer_attr

#[rustfmt::skip]
/// Canary item.
fn value() -> u8 { //~ clippy::missing_const_for_fn
    1
}

fn main() {
    assert!(value() == 1, "one");
}
