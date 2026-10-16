//! Canary.

/// Two fields out of alphabetical order.
struct Rec {
    /// Second letter.
    beta: u8,
    /// First letter.
    alpha: u8, //~ clippy::arbitrary_source_item_ordering
}

/// Entry point.
fn main() {
    let rec = Rec { beta: 2, alpha: 1 };
    assert!(rec.alpha < rec.beta, "ordered");
}
