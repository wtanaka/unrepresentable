//! Canary.
use core::ffi::c_void;

#[rustfmt::skip]
fn main() {
    let boxed: Box<u8> = Box::new(1);
    let raw = Box::into_raw(boxed).cast::<c_void>();
    let back = unsafe { Box::from_raw(raw) }; //~ clippy::from_raw_with_void_ptr unsafe_code clippy::undocumented_unsafe_blocks
    drop(back);
}
