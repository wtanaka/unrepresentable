//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<u8> = Vec::new();
    assert!(items.is_empty()); //~ clippy::assert_is_empty clippy::missing_assert_message
}
