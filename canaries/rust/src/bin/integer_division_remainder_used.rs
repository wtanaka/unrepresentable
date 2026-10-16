//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let count = args().count();
    let rem = count % 2; //~ clippy::integer_division_remainder_used
    exit(i32::from(rem > 1));
}
