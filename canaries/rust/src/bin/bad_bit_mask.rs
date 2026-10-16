//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 3;
    exit(i32::from(val & 1 == 2)); //~ clippy::bad_bit_mask
}
