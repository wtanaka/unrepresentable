//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let opt: Option<String> = black_box(None);
    if let Some(ref text) = opt { //~ clippy::ref_patterns
        sink(text.len());
    }
}
