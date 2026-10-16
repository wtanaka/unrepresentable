//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Empty {} //~ clippy::empty_structs_with_brackets

fn main() {
    assert!(size_of::<Empty>() == 0, "zero");
}
