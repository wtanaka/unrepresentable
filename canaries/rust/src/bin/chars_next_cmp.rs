//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from("str".chars().next() == Some('s'))); //~ clippy::chars_next_cmp
}
