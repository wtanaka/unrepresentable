//! Canary.
/// Canary item.
fn same(items: &[u8]) -> Vec<u8> {
    items.iter().copied().map(|item| item).collect() //~ clippy::map_identity
}

fn main() {
    assert_eq!(same(&[1]).len(), 1, "same");
}
