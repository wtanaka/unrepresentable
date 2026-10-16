//! Canary.
use std::fs;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let path = "a";
    sink(fs::read(&path)); //~ clippy::needless_borrows_for_generic_args
}
