//! Canary.
use std::process::exit;

fn main() {
    let rg = 'a'..'z'; //~ clippy::almost_complete_range
    exit(i32::from(rg.is_empty()));
}
