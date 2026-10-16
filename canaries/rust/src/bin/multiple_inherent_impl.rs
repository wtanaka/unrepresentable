//! Canary.
/// Canary item.
struct Thing;

impl Thing {}

impl Thing {} //~ clippy::multiple_inherent_impl

/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink(Thing);
}
