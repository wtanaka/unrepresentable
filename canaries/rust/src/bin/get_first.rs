//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = vec![1, 2];
    assert!(items.get(0).is_some(), "some"); //~ clippy::get_first
}
