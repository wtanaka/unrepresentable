//! Canary.
use std::fs::create_dir;
use std::process::exit;

fn main() {
    exit(i32::from(create_dir("canary").is_ok())); //~ clippy::create_dir
}
