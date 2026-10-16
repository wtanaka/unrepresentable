//! Canary.
/// Canary item.
fn copy(items: &[u8]) -> Vec<u8> {
    items.iter().map(|item| *item).collect() //~ clippy::map_clone
}

fn main() {
    assert_eq!(copy(&[1]).len(), 1, "copy");
}
