//! Canary.
/// Canary item.
const NAME: &'static str = "a"; //~ clippy::redundant_static_lifetimes

#[rustfmt::skip]
fn main() {
    sink(NAME);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
