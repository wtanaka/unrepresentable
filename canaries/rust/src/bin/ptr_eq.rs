//! Canary.
#[rustfmt::skip]
/// Canary item.
fn sink<T>(_value: T) {}

#[rustfmt::skip]
fn main() {
    let val: u32 = 1;
    let first = &val;
    let second = &val;
    sink(first as *const u32 == second as *const u32); //~ clippy::ptr_eq clippy::as_conversions clippy::ref_as_ptr trivial_casts
}
