//! Canary.
use core::mem::transmute;

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    let ptr: *const u8 = &raw const one;
    // SAFETY: pointer to address conversion.
    let address = unsafe { transmute::<*const u8, usize>(ptr) }; //~ unsafe_code clippy::transmutes_expressible_as_ptr_casts
    assert!(address > 0, "address");
}
