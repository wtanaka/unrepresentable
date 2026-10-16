//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from(u8::MAX as u32 == 2)); //~ clippy::cast_lossless clippy::as_conversions
}
