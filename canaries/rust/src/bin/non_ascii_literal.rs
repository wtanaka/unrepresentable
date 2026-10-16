//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    sink("é"); //~ clippy::non_ascii_literal
}
