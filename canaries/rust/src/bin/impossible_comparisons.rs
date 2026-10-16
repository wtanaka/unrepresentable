//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let count = args().count();
    if count > 5 && count < 3 { //~ clippy::impossible_comparisons
        exit(1);
    }
}
