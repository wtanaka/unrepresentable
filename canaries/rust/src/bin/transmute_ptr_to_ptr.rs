//! Canary.
use core::mem::transmute;

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    let ptr: *const u8 = &raw const one;
    // SAFETY: only the pointee type changes.
    let other = unsafe { transmute::<*const u8, *const i8>(ptr) }; //~ unsafe_code clippy::transmute_ptr_to_ptr
    assert!(!other.is_null(), "non null");
}
