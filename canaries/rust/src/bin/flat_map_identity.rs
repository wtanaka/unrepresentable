//! Canary.
#[rustfmt::skip]
fn main() {
    let items: Vec<Vec<u8>> = vec![vec![1], vec![2]];
    let total = items.into_iter().flat_map(|item| item).count(); //~ clippy::flat_map_identity
    assert!(total == 2, "two");
}
