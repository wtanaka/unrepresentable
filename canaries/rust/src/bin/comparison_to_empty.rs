//! Canary.
use std::process::exit;

fn main() {
    let text = String::new();
    exit(i32::from(text == "")); //~ clippy::comparison_to_empty
}
