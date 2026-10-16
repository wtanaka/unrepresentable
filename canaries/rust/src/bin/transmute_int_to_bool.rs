//! Canary.
use core::mem::transmute;

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    // SAFETY: one is a valid bool representation.
    let flag = unsafe { transmute::<u8, bool>(one) }; //~ unsafe_code clippy::transmute_int_to_bool
    assert!(flag, "flag");
}
