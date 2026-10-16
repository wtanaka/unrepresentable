//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<Option<u8>> = vec![Some(1), None];
    let total = items.into_iter().filter_map(|item| item).count(); //~ clippy::filter_map_identity
    assert!(total == 1, "one");
}
