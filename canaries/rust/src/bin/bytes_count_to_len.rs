//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from("foo".bytes().count() == 2)); //~ clippy::bytes_count_to_len
}
