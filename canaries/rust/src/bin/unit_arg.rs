//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn consume() {}

#[rustfmt::skip]
fn main() {
    let value = Some(consume()); //~ clippy::unit_arg
    assert!(value.is_some(), "some");
}
