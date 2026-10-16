//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let left = args().count();
    let right = args().count();
    if left >= right + 1 { //~ clippy::int_plus_one clippy::arithmetic_side_effects
        exit(1);
    }
}
