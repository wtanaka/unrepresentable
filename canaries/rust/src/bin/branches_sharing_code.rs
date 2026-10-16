//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let mut vec: Vec<u8> = Vec::new();
    if args().count() == 1 {
        vec.push(1);
        vec.push(9);
    } else {
        vec.push(2);
        vec.push(9); //~ clippy::branches_sharing_code
    }
    exit(i32::from(vec.is_empty()));
}
