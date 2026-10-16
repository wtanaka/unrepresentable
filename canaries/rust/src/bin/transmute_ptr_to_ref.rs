//! Canary.
use core::mem::transmute;

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    let ptr: *const u8 = &raw const one;
    // SAFETY: the pointer is valid.
    let reference = unsafe { transmute::<*const u8, &u8>(ptr) }; //~ unsafe_code clippy::transmute_ptr_to_ref
    assert!(*reference > 0, "positive");
}
