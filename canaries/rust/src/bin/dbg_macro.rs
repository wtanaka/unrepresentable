//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from(dbg!(true))); //~ clippy::dbg_macro
}
