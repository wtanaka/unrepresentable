//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let val = args().count() as u32; //~ clippy::cast_possible_truncation clippy::as_conversions
    exit(i32::from(val == 2));
}
