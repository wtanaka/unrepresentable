//! Canary.
#[rustfmt::skip]
fn main() {
    let list: Vec<u8> = Vec::new();
    sink(list.iter().find(|val| **val == 1).is_some()); //~ clippy::search_is_some
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
