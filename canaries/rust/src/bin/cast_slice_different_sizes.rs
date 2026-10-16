//! Canary.
use std::process::exit;

fn main() {
    let arr: [i32; 3] = [1, 2, 3];
    let sl: &[i32] = &arr;
    let src = sl as *const [i32]; //~ clippy::as_conversions trivial_casts clippy::ref_as_ptr
    let dst = src as *const [u8]; //~ clippy::cast_slice_different_sizes clippy::as_conversions
    exit(i32::from(dst.is_null()));
}
