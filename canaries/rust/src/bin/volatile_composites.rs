//! Canary.
use core::ptr;
fn main() {
    let mut src: [u8; 2] = [0, 1];
    // SAFETY: valid
    let _v = unsafe { ptr::read_volatile(&raw mut src) }; //~ clippy::volatile_composites unsafe_code
}
