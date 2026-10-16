//! Canary.
/// Canary item.
struct Pair {
    /// Canary item.
    first: u8,
    /// Canary item.
    second: u8,
}

fn main() {
    let first: u8 = 1;
    let second: u8 = 2;
    let pair = Pair { second, first }; //~ clippy::inconsistent_struct_constructor
    assert_eq!(pair.first, pair.second, "same");
}
