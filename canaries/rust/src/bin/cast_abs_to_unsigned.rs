//! Canary.
use std::process::exit;

fn main() {
    let val: i32 = -3;
    exit(i32::from(val.abs() as u32 == 2)); //~ clippy::cast_abs_to_unsigned clippy::as_conversions clippy::cast_sign_loss
}
