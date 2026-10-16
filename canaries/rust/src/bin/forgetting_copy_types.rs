//! Canary.
use core::mem;

#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    mem::forget(val); //~ forgetting_copy_types
}
