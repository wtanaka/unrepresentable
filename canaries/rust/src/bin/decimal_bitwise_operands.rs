//! Canary.
use std::process::exit;

fn main() {
    let val = u32::from(true);
    exit(i32::from(val & 12345 == 0)); //~ clippy::decimal_bitwise_operands
}
