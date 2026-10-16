//! Canary.
#[rustfmt::skip]
#[derive(Default)]
/// Canary item.
struct Pair {
    /// Canary item.
    left: u8, //~ dead_code
    /// Canary item.
    right: u8,
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let pair = Pair {
        left: 1,
        right: 2,
        ..Default::default() //~ clippy::needless_update
    };
    sink(pair);
}
