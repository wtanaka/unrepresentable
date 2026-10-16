//! Canary.
use core::mem::transmute;
use core::ptr::null;

#[rustfmt::skip]
fn main() {
    // SAFETY: never used.
    let reference = unsafe { transmute::<*const u8, &u8>(null()) }; //~ unsafe_code clippy::transmuting_null clippy::transmute_ptr_to_ref
    assert!(*reference > 0, "positive");
}
