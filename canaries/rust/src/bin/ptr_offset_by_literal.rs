//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u32 = 1;
    let raw: *const u32 = &raw const val;
    sink(raw.wrapping_offset(1)); //~ clippy::ptr_offset_by_literal
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
