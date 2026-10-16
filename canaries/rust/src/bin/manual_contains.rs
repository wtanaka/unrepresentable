//! Canary.
/// Canary item.
fn has(items: &[u8]) -> bool {
    items.iter().any(|item| *item == 3) //~ clippy::manual_contains
}

fn main() {
    assert!(has(&[3]), "has");
}
