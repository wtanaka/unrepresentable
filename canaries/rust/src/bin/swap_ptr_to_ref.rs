//! Canary.
use core::mem::swap;

#[rustfmt::skip]
fn main() {
    let mut first: u8 = 1;
    let mut second: u8 = 2;
    let left: *mut u8 = &raw mut first;
    let right: *mut u8 = &raw mut second;
    // SAFETY: both pointers are valid and distinct.
    unsafe { swap(&mut *left, &mut *right) } //~ unsafe_code clippy::swap_ptr_to_ref clippy::multiple_unsafe_ops_per_block
    assert!(first > second, "swapped");
}
