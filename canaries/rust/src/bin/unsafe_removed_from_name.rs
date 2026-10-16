//! Canary.
use core::cell::UnsafeCell as Cell; //~ clippy::unsafe_removed_from_name
fn main() {
    let value: i32 = 1;
    let _v = Cell::new(value);
}
