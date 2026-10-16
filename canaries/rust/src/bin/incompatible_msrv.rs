//! Canary.
use std::process::exit;

#[clippy::msrv = "1.50.0"]
fn main() {
    let left: u8 = 3;
    let right: u8 = 5;
    let diff = left.abs_diff(right); //~ clippy::incompatible_msrv
    exit(i32::from(diff));
}
