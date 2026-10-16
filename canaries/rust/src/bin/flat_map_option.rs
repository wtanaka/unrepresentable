//! Canary.
#[rustfmt::skip]
fn main() {
    let items: [u8; 2] = [1, 2];
    let total = items.iter().flat_map(|item| item.checked_add(1)).count(); //~ clippy::flat_map_option
    assert!(total == 2, "two");
}
