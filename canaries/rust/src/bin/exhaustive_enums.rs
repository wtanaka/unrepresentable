//! Canary.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
/// Canary item.
pub enum Choice { //~ clippy::exhaustive_enums
    /// Canary item.
    First,
    /// Canary item.
    Second,
}

fn main() {
    assert!(matches!(Choice::First, Choice::First), "first");
    assert!(!matches!(Choice::Second, Choice::First), "second");
}
