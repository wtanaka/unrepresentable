//! Canary.
/// Canary item.
fn any(items: &[u8]) -> bool {
    items.iter().map(|item| *item > 1).any(|flag| flag) //~ clippy::map_all_any_identity
}

fn main() {
    assert!(any(&[2]), "any");
}
