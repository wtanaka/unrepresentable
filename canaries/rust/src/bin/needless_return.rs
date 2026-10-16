//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

/// Canary item.
const fn one() -> u8 {
    return 1; //~ clippy::needless_return
}
fn main() {
    sink(one());
}
