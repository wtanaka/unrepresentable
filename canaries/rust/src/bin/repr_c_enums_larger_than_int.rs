//! Canary.
use core::hint::black_box;

#[repr(C)]
/// Canary item.
enum Big {
    Huge = 0x1_0000_0000, //~ repr_c_enums_larger_than_int clippy::enum_clike_unportable_variant
}

fn main() {
    let _: Big = black_box(Big::Huge);
}
