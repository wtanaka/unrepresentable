//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Pair {
    /// Canary item.
    left: u8,
}

#[rustfmt::skip]
fn main() {
    let left: u8 = 1;
    let pair = Pair { left: left }; //~ clippy::redundant_field_names
    sink(pair.left);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
