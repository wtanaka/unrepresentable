//! Canary.
use std::process::exit;

/// Canary item.
enum Wrap {
    /// Canary item.
    Yes(u32), //~ dead_code
}

fn main() {
    let val = Wrap::Yes as usize; //~ clippy::cast_enum_constructor clippy::as_conversions function_casts_as_integer clippy::fn_to_numeric_cast_any
    exit(i32::from(val == 2));
}
