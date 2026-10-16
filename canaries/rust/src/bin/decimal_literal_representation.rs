//! Canary.
use std::process::exit;

fn main() {
    let val: u32 = 65_536; //~ clippy::decimal_literal_representation
    exit(i32::from(val == 2));
}
