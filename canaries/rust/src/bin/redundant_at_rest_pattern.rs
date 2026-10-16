//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let arr: [u8; 2] = [1, 2];
    let [all @ ..] = arr; //~ clippy::redundant_at_rest_pattern
    sink(all);
}
