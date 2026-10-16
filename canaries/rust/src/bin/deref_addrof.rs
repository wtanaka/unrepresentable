//! Canary.
use std::process::exit;

fn main() {
    let val: u8 = 1;
    exit(i32::from(*&val == 2)); //~ clippy::deref_addrof
}
