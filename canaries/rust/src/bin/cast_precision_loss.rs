//! Canary.
use std::env::args;
use std::process::exit;

fn main() {
    let val = args().count() as f64; //~ clippy::cast_precision_loss clippy::as_conversions
    exit(i32::from(val.is_nan()));
}
