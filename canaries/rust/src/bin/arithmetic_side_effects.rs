//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let num = args().count();
    exit(i32::from(num + num == 2)); //~ clippy::arithmetic_side_effects
}
