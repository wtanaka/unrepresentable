//! Canary.
use std::env::args;
use std::process::exit;

#[rustfmt::skip]
fn main() {
    let count = args().count();
    let res: i32 = if count > 1 { 1 } else if count > 1 { 2 } else { 3 }; //~ clippy::ifs_same_cond
    exit(res);
}
