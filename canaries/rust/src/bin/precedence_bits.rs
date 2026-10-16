//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let one: u32 = black_box(1);
    let three: u32 = black_box(3);
    sink(one & one << three); //~ clippy::precedence_bits
}
