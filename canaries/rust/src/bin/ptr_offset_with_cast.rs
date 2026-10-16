//! Canary.
use core::hint::black_box;

#[rustfmt::skip]
fn main() {
    let val: u32 = 1;
    let raw: *const u32 = &raw const val;
    let off: usize = black_box(1);
    sink(raw.wrapping_offset(off as isize)); //~ clippy::ptr_offset_with_cast clippy::cast_possible_wrap clippy::as_conversions
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
