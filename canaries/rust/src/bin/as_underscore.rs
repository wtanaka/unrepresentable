//! Canary.
use std::process::exit;

fn main() {
    let val: u32 = u8::MAX as _; //~ clippy::as_underscore clippy::as_conversions clippy::cast_lossless
    exit(i32::from(val == 2));
}
