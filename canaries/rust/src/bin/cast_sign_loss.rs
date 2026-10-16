//! Canary.
use std::process::exit;

fn main() {
    let val = i32::from(true);
    exit(i32::from(val as u32 == 2)); //~ clippy::cast_sign_loss clippy::as_conversions
}
