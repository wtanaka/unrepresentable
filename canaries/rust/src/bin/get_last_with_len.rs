//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = vec![1, 2];
    assert!(items.get(items.len() - 1).is_some(), "some"); //~ clippy::get_last_with_len clippy::arithmetic_side_effects
}
