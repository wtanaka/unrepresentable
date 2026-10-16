//! Canary.
use std::process::exit;

fn main() {
    let val: String = Default::default(); //~ clippy::default_trait_access
    exit(i32::from(val.is_empty()));
}
