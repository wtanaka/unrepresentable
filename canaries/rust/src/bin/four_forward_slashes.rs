//! Canary.
//// Not a doc comment. //~ clippy::four_forward_slashes
#[rustfmt::skip]
/// Canary item.
fn value() -> u8 { //~ clippy::missing_const_for_fn
    1
}

fn main() {
    assert!(value() == 1, "one");
}
