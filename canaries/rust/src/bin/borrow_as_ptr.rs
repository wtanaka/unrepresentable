//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 1;
    let ptr = &val as *const u8; //~ clippy::borrow_as_ptr trivial_casts clippy::as_conversions raw_borrows_via_references
    exit(i32::from(ptr.is_null()));
}
