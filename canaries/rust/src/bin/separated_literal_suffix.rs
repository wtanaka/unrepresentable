//! Canary.
#[rustfmt::skip]
fn main() {
    sink(1_u8); //~ clippy::separated_literal_suffix
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
