//! Canary.
extern crate alloc;
use alloc::alloc::Layout; //~ clippy::alloc_instead_of_core
use std::process::exit;

fn main() {
    exit(i32::from(Layout::new::<u8>().size() == 0));
}
