//! Canary.
use core::hint::black_box;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let mut flag = false; //~ unused_assignments
    let cond = black_box(true);
    if cond { flag = true } else { flag = false } //~ clippy::needless_bool_assign
    sink(flag);
}
