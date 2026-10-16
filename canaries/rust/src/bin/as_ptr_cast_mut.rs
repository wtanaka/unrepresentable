//! Canary.
use std::process::exit;

fn main() {
    let text = String::new();
    let ptr = text.as_ptr() as *mut u8; //~ clippy::as_ptr_cast_mut clippy::ptr_cast_constness clippy::as_conversions
    exit(i32::from(ptr.is_null()));
}
