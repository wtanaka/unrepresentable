//! Canary.
#[rustfmt::skip]
/// Canary item.
enum Choice {
    /// Canary item.
    Other,
    /// Canary item.
    Unit(), //~ clippy::empty_enum_variants_with_brackets
}

fn main() {
    let choice = Choice::Unit();
    assert!(matches!(choice, Choice::Unit()), "unit");
    assert!(!matches!(Choice::Other, Choice::Unit()), "other");
}
