//! Canary.
#[rustfmt::skip]
fn main() {
    let list: Vec<String> = Vec::new();
    sink(list.iter().cloned().count()); //~ clippy::redundant_iter_cloned
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
