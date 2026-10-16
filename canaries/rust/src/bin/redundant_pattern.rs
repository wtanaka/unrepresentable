//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let opt: Option<u8> = black_box(None);
    match opt {
        Some(val @ _) => sink(val), //~ clippy::redundant_pattern
        None => sink::<u8>(0),
    }
}
