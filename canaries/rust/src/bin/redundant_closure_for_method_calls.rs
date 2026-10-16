//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let list: Vec<String> = Vec::new();
    sink(list.iter().map(|text| text.len()).max()); //~ clippy::redundant_closure_for_method_calls
}
