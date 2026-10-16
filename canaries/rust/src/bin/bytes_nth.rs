//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from("str".bytes().nth(3).is_none())); //~ clippy::bytes_nth
}
