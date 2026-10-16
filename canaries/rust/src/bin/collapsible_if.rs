//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let num = args().count();
    if num == 1 { //~ clippy::collapsible_if
        if num == 2 {
            exit(2);
        }
    }
}
