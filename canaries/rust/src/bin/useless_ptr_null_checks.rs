//! Canary.
use core::hint::black_box;
use core::ptr;

fn main() {
    let value: u8 = 1;
    let _: bool = black_box(ptr::from_ref(&value).is_null()); //~ useless_ptr_null_checks
}
