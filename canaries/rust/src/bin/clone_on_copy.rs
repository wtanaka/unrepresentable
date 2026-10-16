//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 1;
    exit(i32::from(val.clone() == 2)); //~ clippy::clone_on_copy
}
