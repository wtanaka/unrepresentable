//! Canary.
use std::process::exit;

fn main() {
    let val: f64 = 3.14159; //~ clippy::approx_constant
    exit(i32::from(val.is_nan()));
}
