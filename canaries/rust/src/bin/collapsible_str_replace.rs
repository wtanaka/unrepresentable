//! Canary.
use std::process::exit;

#[rustfmt::skip]
fn main() {
    exit(i32::from("str".replace('a', "x").replace('b', "x").is_empty())); //~ clippy::collapsible_str_replace
}
