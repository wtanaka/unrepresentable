//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let cond = black_box(true);
    let val: u8; //~ clippy::useless_let_if_seq clippy::needless_late_init
    if cond {
        val = 1;
    } else {
        val = 2;
    }
    sink(val);
}
