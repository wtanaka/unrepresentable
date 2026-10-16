//! Canary.
extern crate alloc;
use alloc::sync::Arc;
use core::cell::RefCell;
use std::process::exit;

fn main() {
    let cell: RefCell<u8> = RefCell::new(1);
    let arc = Arc::new(cell); //~ clippy::arc_with_non_send_sync
    exit(i32::from(Arc::strong_count(&arc) > 1));
}
