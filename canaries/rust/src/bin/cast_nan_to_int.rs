//! Canary.
use std::process::exit;

fn main() {
    let val = (f64::INFINITY + f64::NEG_INFINITY) as usize; //~ clippy::cast_nan_to_int clippy::as_conversions clippy::cast_sign_loss clippy::cast_possible_truncation clippy::float_arithmetic
    exit(i32::from(val == 2));
}
