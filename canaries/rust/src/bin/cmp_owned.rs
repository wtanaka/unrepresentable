//! Canary.
use std::process::exit;

fn main() {
    let text = "x";
    exit(i32::from(text.to_string() == "y")); //~ clippy::cmp_owned clippy::str_to_string
}
