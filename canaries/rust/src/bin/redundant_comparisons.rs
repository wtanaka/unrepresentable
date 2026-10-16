//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let num: u8 = black_box(1);
    sink(num > 3 && num > 2); //~ clippy::redundant_comparisons
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
