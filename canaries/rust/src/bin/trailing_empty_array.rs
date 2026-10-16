//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Tail { //~ clippy::trailing_empty_array
    /// Canary item.
    head: u8,
    /// Canary item.
    tail: [u8; 0], //~ clippy::struct_field_names
}

#[rustfmt::skip]
fn main() {
    let value = Tail { head: 1, tail: [] };
    assert!(value.head > 0 && value.tail.is_empty(), "positive");
}
