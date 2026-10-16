//! Canary.
use std::process::exit;

fn main() {
    let val = 1; //~ clippy::default_numeric_fallback
    exit(i32::from(val > 0)); //~ clippy::default_numeric_fallback
}
