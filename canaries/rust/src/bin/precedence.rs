//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let one: u32 = black_box(1);
    let two: u32 = black_box(2);
    sink(one << two + one); //~ clippy::precedence clippy::arithmetic_side_effects
}
