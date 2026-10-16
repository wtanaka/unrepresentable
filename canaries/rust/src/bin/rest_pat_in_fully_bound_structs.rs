//! Canary.
#[rustfmt::skip]
/// Canary item.
struct Pair {
    /// Canary item.
    left: u8,
    /// Canary item.
    right: u8,
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let pair = Pair { left: 1, right: 2 };
    let Pair { left, right, .. } = pair; //~ clippy::rest_pat_in_fully_bound_structs
    sink((left, right));
}
