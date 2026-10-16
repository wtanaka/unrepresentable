//! Canary.
use core::hint::black_box;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

#[rustfmt::skip]
fn main() {
    let cond = black_box(true);
    if cond { sink(cond); } else {} //~ clippy::needless_else
}
