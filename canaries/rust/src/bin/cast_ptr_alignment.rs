//! Canary.
use core::ptr::from_ref;
use std::process::exit;

fn main() {
    let byte: u8 = 1;
    let ptr = from_ref(&byte) as *const u16; //~ clippy::cast_ptr_alignment clippy::ptr_as_ptr clippy::as_conversions
    exit(i32::from(ptr.is_null()));
}
