//! Canary.
use core::mem;

/// Canary item.
fn sink<Ty>(_val: Ty) {}

#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    sink(mem::discriminant(&val)); //~ enum_intrinsics_non_enums
}
