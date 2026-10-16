//! Canary.
/// Canary item.
fn many() -> Vec<u8> {
    (0..3).map(|_| 5).collect() //~ clippy::map_with_unused_argument_over_ranges
}

fn main() {
    assert_eq!(many().len(), 3, "many");
}
