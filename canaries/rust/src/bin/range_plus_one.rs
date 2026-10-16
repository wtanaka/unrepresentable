//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let end: usize = black_box(3);
    let arr: [u8; 4] = [1, 1, 1, 1];
    sink(&arr[0..end + 1]); //~ clippy::range_plus_one clippy::arithmetic_side_effects clippy::indexing_slicing
}
