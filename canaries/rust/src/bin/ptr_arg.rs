//! Canary.
#[rustfmt::skip]
/// Canary item.
const fn total(items: &Vec<u8>) -> usize { //~ clippy::ptr_arg
    items.len()
}

#[rustfmt::skip]
fn main() {
    sink(total(&Vec::new()));
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
