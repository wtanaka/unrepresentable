//! Canary.
use core::ptr::null;
use std::process::exit;

fn main() {
    let val: u8 = 1;
    let ptr: *const u8 = &raw const val;
    exit(i32::from(ptr == null())); //~ clippy::cmp_null
}
