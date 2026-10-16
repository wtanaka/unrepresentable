//! Canary.
/// Canary item.
fn last(items: &[u8]) -> Option<&u8> {
    items.iter().rev().next() //~ clippy::manual_next_back
}

fn main() {
    assert_eq!(last(&[1]), Some(&1), "last");
}
