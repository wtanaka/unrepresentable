//! Canary.
/// Canary item.
const fn small(value: u8) -> bool {
    match value {
        1 | 2 | 3 => true, //~ clippy::manual_range_patterns
        _ => false,
    }
}

fn main() {
    assert!(small(2), "small");
}
