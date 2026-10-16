//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let val = args().count() as isize; //~ clippy::cast_possible_wrap clippy::as_conversions
    exit(i32::from(val == 2));
}
