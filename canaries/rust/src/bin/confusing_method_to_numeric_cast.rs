//! Canary.
use std::process::exit;

fn main() {
    let val = f32::max as usize; //~ clippy::confusing_method_to_numeric_cast clippy::as_conversions function_casts_as_integer clippy::fn_to_numeric_cast_any
    exit(i32::from(val == 2));
}
