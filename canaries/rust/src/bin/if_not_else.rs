//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let count = args().count();
    let res: i32 = if count != 1 { 3 } else { 4 }; //~ clippy::if_not_else
    exit(res);
}
