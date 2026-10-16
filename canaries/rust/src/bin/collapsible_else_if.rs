//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let num = args().count();
    if num == 1 {
        exit(1);
    } else { //~ clippy::collapsible_else_if
        if num == 2 {
            exit(2);
        }
    }
}
