//! Canary.
use core::ptr;
fn main() {
    let _v = ptr::null::<()>().wrapping_add(1); //~ clippy::zst_offset
}
