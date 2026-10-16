//! Canary.
#[rustfmt::skip]
#[derive(Debug, Clone, Copy)]
/// Canary item.
pub struct Pair {
    /// Canary item.
    pub left: u8,
    /// Canary item.
    right: u8, //~ clippy::partial_pub_fields
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let pair = Pair { left: 1, right: 2 };
    sink(pair.left);
    sink(pair.right);
    sink(pair);
}
