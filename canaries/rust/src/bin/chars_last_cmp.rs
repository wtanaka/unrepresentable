//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from("str".chars().last() == Some('r'))); //~ clippy::chars_last_cmp
}
