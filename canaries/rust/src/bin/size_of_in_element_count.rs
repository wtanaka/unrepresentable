//! Canary.
use core::ptr::copy_nonoverlapping;

#[rustfmt::skip]
fn main() {
    let src: [u32; 4] = [1; 4];
    let mut dst: [u32; 4] = [0; 4];
    let count: usize = 4;
    // SAFETY: both buffers hold four u32 values and do not overlap.
    unsafe { copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), count * size_of::<u32>()); } //~ unsafe_code clippy::size_of_in_element_count clippy::arithmetic_side_effects clippy::semicolon_outside_block
    assert!(dst.is_empty(), "empty");
}
