//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    for val in 5_u8..1 { //~ clippy::reversed_empty_ranges clippy::separated_literal_suffix
        sink(val);
    }
}
