//! Canary.
/// Canary item.
fn bump(items: &[u8]) -> Vec<u8> {
    items
        .iter()
        .filter(|num| num.checked_add(1).is_some()) //~ clippy::manual_filter_map
        .map(|num| num.checked_add(1).unwrap()) //~ clippy::unwrap_used
        .collect()
}

fn main() {
    assert_eq!(bump(&[1]).len(), 1, "bump");
}
