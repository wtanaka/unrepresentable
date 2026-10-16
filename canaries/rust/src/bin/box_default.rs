//! Canary.
use std::process::exit;

fn main() {
    let bx: Box<String> = Box::new(String::default()); //~ clippy::box_default
    exit(i32::from(bx.is_empty()));
}
