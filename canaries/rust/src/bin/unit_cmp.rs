//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn consume() {}

#[rustfmt::skip]
fn main() {
    let same = consume() == (); //~ clippy::unit_cmp
    assert!(same, "same");
}
