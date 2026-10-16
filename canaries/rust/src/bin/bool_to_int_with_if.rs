//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let flag = args().count() == 1;
    let num: i32 = if flag { 1 } else { 0 }; //~ clippy::bool_to_int_with_if
    exit(num);
}
