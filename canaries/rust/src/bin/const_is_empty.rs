//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from("".is_empty())); //~ clippy::const_is_empty
}
