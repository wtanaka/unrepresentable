//! Canary.
use core::mem;

#[rustfmt::skip]
fn main() {
    // SAFETY: canary only.
    let val: &u8 = unsafe { mem::zeroed() }; //~ unsafe_code invalid_value
    assert_eq!(*val, 0, "value");
}
