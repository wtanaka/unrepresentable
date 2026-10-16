//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let count = args().count();
    if count | 1 > 3 { //~ clippy::ineffective_bit_mask
        exit(1);
    }
}
