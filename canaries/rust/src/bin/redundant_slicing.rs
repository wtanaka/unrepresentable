//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let list: &[u8] = &[];
    sink(&list[..]); //~ clippy::redundant_slicing
}
