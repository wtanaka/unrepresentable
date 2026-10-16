//! Canary.

/// A record with a crate-scoped field.
struct Rec {
    /// The only field.
    pub(crate) only: u8, //~ clippy::field_scoped_visibility_modifiers
}

/// Entry point.
fn main() {
    let rec = Rec { only: 1 };
    assert_eq!(rec.only, 1, "one");
}
