//! Canary.
use core::ptr;

#[rustfmt::skip]
fn main() {
    let num: u8 = 1;
    // SAFETY: canary only.
    unsafe { *ptr::from_ref(&num).cast_mut() = 2 }; //~ unsafe_code invalid_reference_casting clippy::semicolon_inside_block
}
