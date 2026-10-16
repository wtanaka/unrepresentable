//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let left = args().count();
    let right = args().count();
    let diff = if left > right { left - right } else { right - left }; //~ clippy::manual_abs_diff clippy::arithmetic_side_effects
    exit(i32::from(diff > 5));
}
