//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let count = args().count();
    let half = count / 2; //~ clippy::integer_division clippy::integer_division_remainder_used
    exit(i32::from(half > 1));
}
