//! Canary.
use std::fs::OpenOptions;

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink(OpenOptions::new().read(true).truncate(true).open("x")); //~ clippy::nonsensical_open_options
}
