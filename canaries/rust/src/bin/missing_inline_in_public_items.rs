//! Canary.
#[rustfmt::skip]
/// Canary item.
pub fn one() -> u8 { //~ clippy::missing_const_for_fn clippy::missing_inline_in_public_items clippy::must_use_candidate
    1
}

fn main() {
    assert_eq!(one(), 1, "one");
}
