//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u32 = 1;
    let raw: *const u32 = &raw const val;
    sink(raw as *mut u32); //~ clippy::ptr_cast_constness clippy::as_conversions
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
