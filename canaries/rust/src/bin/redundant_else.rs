//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let num: u8 = black_box(1);
    loop {
        if num == 1 {
            break;
        } else { //~ clippy::redundant_else
            sink(num);
        }
    }
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
