//! Canary.
#[rustfmt::skip]
/// Canary item.
fn size(text: &str) -> usize {
    text.chars().count()
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let list: Vec<&str> = Vec::new();
    sink(list.into_iter().map(|text| size(text)).max()); //~ clippy::redundant_closure
}
