//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let flag = args().count() == 1;
    exit(i32::from(flag == true)); //~ clippy::bool_comparison
}
