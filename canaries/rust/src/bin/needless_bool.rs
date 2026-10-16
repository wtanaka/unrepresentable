//! Canary.
use core::hint::black_box;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let cond = black_box(true);
    let res = if cond { true } else { false }; //~ clippy::needless_bool
    sink(res);
}
