//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let text = String::new();
    sink(text.as_bytes().len()); //~ clippy::needless_as_bytes
}
