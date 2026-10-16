//! Canary.
/// Canary item.
fn keep(mut items: Vec<u8>) -> Vec<u8> {
    items = items.into_iter().filter(|item| *item > 1).collect(); //~ clippy::manual_retain
    items
}

fn main() {
    assert_eq!(keep(vec![1, 2]).len(), 1, "keep");
}
