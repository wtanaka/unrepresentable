//! Canary.
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let val: u8 = 1;
    if { let inner = val; inner == 1 } { //~ clippy::blocks_in_conditions
        exit(1);
    }
}
