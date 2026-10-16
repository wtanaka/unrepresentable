//! Canary.
#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    assert!(items.iter().rev().last().is_some(), "some"); //~ clippy::double_ended_iterator_last
}
