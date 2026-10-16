//! Canary.
/// Canary item.
fn first(items: &[Option<u8>]) -> Option<u8> {
    items
        .iter()
        .find(|item| item.is_some()) //~ clippy::manual_find_map
        .map(|item| item.unwrap()) //~ clippy::unwrap_used
}

fn main() {
    assert_eq!(first(&[Some(1)]), Some(1), "first");
}
