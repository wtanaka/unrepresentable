//! Canary.
use std::process::exit;

fn main() {
    let val = i64::from(true);
    exit(i32::from(val <= i64::from(u16::MAX) && val >= 0)); //~ clippy::checked_conversions
}
