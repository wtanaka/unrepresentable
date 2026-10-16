//! Canary.
/// Canary item.
fn flat(items: &[Option<u8>]) -> Vec<u8> {
    items.iter().map(|item| *item).flatten().collect() //~ clippy::map_clone clippy::map_flatten
}

fn main() {
    assert_eq!(flat(&[Some(1)]).len(), 1, "flat");
}
