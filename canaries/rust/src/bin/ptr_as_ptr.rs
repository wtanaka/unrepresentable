//! Canary.
#[rustfmt::skip]
fn main() {
    let val: u32 = 1;
    let raw = &raw const val;
    sink(raw as *const u8); //~ clippy::ptr_as_ptr clippy::as_conversions
}

#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}
