//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let left = args().count();
    let right = args().count();
    let diff = if left > right { right - left } else { 0 }; //~ clippy::inverted_saturating_sub clippy::arithmetic_side_effects
    exit(i32::from(diff > 0));
}
