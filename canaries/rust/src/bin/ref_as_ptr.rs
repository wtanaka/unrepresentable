//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    let reference = &val;
    let raw = reference as *const u8; //~ clippy::ref_as_ptr clippy::as_conversions trivial_casts
    sink(raw);
}
