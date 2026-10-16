//! Canary.
use core::mem::transmute;
use core::num::NonZeroU8;

#[rustfmt::skip]
fn main() {
    let one: u8 = 1;
    // SAFETY: one is non-zero.
    let value = unsafe { transmute::<u8, NonZeroU8>(one) }; //~ unsafe_code clippy::transmute_int_to_non_zero
    assert!(value.get() > 0, "positive");
}
