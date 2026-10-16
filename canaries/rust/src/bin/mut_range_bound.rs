//! Canary.
use core::hint::black_box;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let mut end: u32 = black_box(10);
    for idx in 0..end {
        end = 5; //~ clippy::mut_range_bound
        sink(idx);
    }
    sink(end);
}
