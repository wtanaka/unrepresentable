//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let num: u8 = black_box(1);
    if num == 1 {
        sink(num);
    } if num == 2 { //~ clippy::possible_missing_else
        sink(num);
    }
}
