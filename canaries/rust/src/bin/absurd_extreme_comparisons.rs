//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 3;
    exit(i32::from(val < 0)); //~ clippy::absurd_extreme_comparisons unused_comparisons
}
