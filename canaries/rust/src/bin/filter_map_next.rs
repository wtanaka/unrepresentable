//! Canary.
#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    let first = items.iter().filter_map(|item| item.checked_add(1)).next(); //~ clippy::filter_map_next
    assert!(first.is_some(), "some");
}
