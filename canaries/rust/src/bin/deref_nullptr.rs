//! Canary.
use core::ptr;

#[rustfmt::skip]
fn main() {
    // SAFETY: canary only.
    let val: u8 = unsafe { *ptr::null::<u8>() }; //~ unsafe_code deref_nullptr
    assert_eq!(val, 0, "value");
}
