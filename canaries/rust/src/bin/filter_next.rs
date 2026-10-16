//! Canary.
#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    let first = items.iter().filter(|item| **item > 1).next(); //~ clippy::filter_next
    assert!(first.is_some(), "some");
}
