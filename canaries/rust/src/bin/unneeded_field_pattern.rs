//! Canary.
/// Canary item.
struct Holder {
    /// Canary item.
    count: i32,
    /// Canary item.
    other: i32,
}
fn main() {
    let src = Holder { count: 1, other: 2 };
    let _w = src.other.to_string();
    let Holder { count, other: _ } = src; //~ clippy::unneeded_field_pattern
    let _v = count.to_string();
}
