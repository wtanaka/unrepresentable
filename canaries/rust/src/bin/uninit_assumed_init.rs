//! Canary.
use core::mem::MaybeUninit;

#[rustfmt::skip]
fn main() {
    // SAFETY: not sound, never relied upon.
    let value: u8 = unsafe { MaybeUninit::uninit().assume_init() }; //~ unsafe_code clippy::uninit_assumed_init invalid_value
    assert!(value > 0, "positive");
}
