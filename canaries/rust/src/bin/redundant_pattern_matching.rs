//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let opt: Option<u8> = black_box(None);
    if let Some(_) = opt { //~ clippy::redundant_pattern_matching
        sink(opt);
    }
}
