//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let num = args().count();
    if num > 2 { //~ clippy::comparison_chain
        exit(1);
    } else if num < 2 {
        exit(2);
    } else {
        exit(3);
    }
}
