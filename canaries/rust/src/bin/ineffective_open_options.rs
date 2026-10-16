//! Canary.
use std::fs::OpenOptions;
use std::process::exit;

fn main() {
    let file = OpenOptions::new().write(true).append(true).open("out.txt"); //~ clippy::ineffective_open_options
    exit(i32::from(file.is_ok()));
}
