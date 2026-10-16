//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn check() -> bool { //~ clippy::missing_const_for_fn
    black_box(true)
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    if check() { //~ clippy::same_functions_in_if_condition
        sink::<u8>(1);
    } else if check() {
        sink::<u8>(2);
    } else {
        sink::<u8>(3);
    }
}
