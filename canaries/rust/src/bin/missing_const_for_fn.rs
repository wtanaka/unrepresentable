//! Canary.
#[rustfmt::skip]
/// Canary item.
fn one() -> u8 { //~ clippy::missing_const_for_fn
    1
}

fn main() {
    assert_eq!(one(), 1, "one");
}
