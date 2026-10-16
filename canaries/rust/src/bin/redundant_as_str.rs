//! Canary.
#[rustfmt::skip]
fn main() {
    let text = String::from("a");
    sink(text.as_str().as_bytes()); //~ clippy::redundant_as_str
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
