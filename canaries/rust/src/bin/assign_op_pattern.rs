//! Canary.
use std::process::exit;

fn main() {
    let mut num: u8 = 0;
    num = num | 1; //~ clippy::assign_op_pattern
    exit(i32::from(num == 2));
}
