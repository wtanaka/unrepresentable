//! Canary.
use core::hint::black_box;

/// Canary item.
const fn first() {}

fn main() {
    let left: fn() = first;
    let right: fn() = first;
    let _: bool = black_box(left == right); //~ unpredictable_function_pointer_comparisons
}
