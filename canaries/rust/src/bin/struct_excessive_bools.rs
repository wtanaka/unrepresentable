//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Flags { //~ clippy::struct_excessive_bools
    /// Canary item.
    first: bool,
}

#[rustfmt::skip]
fn main() {
    let flags = Flags { first: true };
    assert!(flags.first, "first");
}
