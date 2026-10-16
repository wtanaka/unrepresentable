//! Canary.
use std::process::exit;

fn main() {
    let val: f64 = f64::from(u8::MAX);
    let res: f64 = val.exp() - 1.0; //~ clippy::imprecise_flops clippy::float_arithmetic
    exit(i32::from(res > val));
}
