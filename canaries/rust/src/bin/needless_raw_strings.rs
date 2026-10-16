//! Canary.
/// Canary item.
fn sink<T>(value: T) {
    drop(value);
}

fn main() {
    let text = r"abc"; //~ clippy::needless_raw_strings
    sink(text);
}
