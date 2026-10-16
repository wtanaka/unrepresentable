//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from('a' as u8 == 2)); //~ clippy::char_lit_as_u8 clippy::as_conversions
}
