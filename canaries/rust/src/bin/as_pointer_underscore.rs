//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 1;
    let ptr: *const u8 = &val as *const _; //~ clippy::as_pointer_underscore clippy::borrow_as_ptr trivial_casts clippy::as_conversions
    exit(i32::from(ptr.is_null()));
}
