//! Canary.
use std::cmp::Ordering; //~ clippy::std_instead_of_core

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    assert_eq!(one.cmp(&2), Ordering::Less, "less");
}
