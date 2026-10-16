//! Canary.
use std::process::exit;

fn main() {
    exit(i32::from(u8::MAX as u32 == 2)); //~ clippy::as_conversions clippy::cast_lossless
}
