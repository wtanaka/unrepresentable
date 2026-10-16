//! Canary.
use std::process::exit;

fn main() {
    let val: u32 = 1_23_456; //~ clippy::inconsistent_digit_grouping
    exit(i32::from(val > 5));
}
