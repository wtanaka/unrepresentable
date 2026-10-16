//! Canary.
extern crate alloc;
use alloc::rc::Rc;
use std::process::exit;

fn main() {
    let rc = Rc::new(String::new());
    let other = rc.clone(); //~ clippy::clone_on_ref_ptr
    exit(i32::from(other.is_empty() && rc.is_empty()));
}
