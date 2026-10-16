//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let num: u8 = black_box(1);
    let num = num; //~ clippy::redundant_locals clippy::shadow_same
    sink(num);
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
